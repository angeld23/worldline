use super::texture::Texture;
use super::vertex::Vertex2D;
use crate::gui::color::GuiColor;
use crate::shared::bounding_box::bbox;
use anyhow::{anyhow, Result};
use cgmath::{vec2, Vector2};
use futures::channel::oneshot;
use futures::executor;
use image::RgbaImage;
use linear_map::LinearMap;
use std::cell::Cell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;
use std::{mem, ops::Range};
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, window::Window};

pub type BindGroupFormat = [(wgpu::ShaderStages, wgpu::BindingType)];

pub fn bind_group_format_to_layout_entries(
    format: &BindGroupFormat,
) -> Vec<wgpu::BindGroupLayoutEntry> {
    format
        .iter()
        .copied()
        .enumerate()
        .map(|(i, (stages, binding_type))| wgpu::BindGroupLayoutEntry {
            binding: i as u32,
            visibility: stages,
            ty: binding_type,
            count: None,
        })
        .collect()
}

#[derive(Debug)]
pub struct GpuHandle {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl GpuHandle {
    pub fn create_bind_group_layout(&self, format: &BindGroupFormat) -> wgpu::BindGroupLayout {
        self.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &bind_group_format_to_layout_entries(format),
            })
    }

    pub fn create_bind_group(
        &self,
        layout: &wgpu::BindGroupLayout,
        resources: Vec<wgpu::BindingResource>,
    ) -> wgpu::BindGroup {
        let entries: Vec<wgpu::BindGroupEntry<'_>> = resources
            .into_iter()
            .enumerate()
            .map(|(index, binding_resource)| wgpu::BindGroupEntry {
                binding: index as u32,
                resource: binding_resource,
            })
            .collect();

        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout,
            entries: &entries,
        })
    }

    pub fn binded_texture(
        &self,
        layout: &wgpu::BindGroupLayout,
        texture: Texture,
    ) -> BindedTexture {
        let bind_group = self.create_bind_group(
            layout,
            vec![
                wgpu::BindingResource::TextureView(&texture.view),
                wgpu::BindingResource::Sampler(&texture.sampler),
            ],
        );
        BindedTexture {
            texture,
            bind_group,
        }
    }

    pub fn binded_buffer<T>(
        &self,
        layout: &wgpu::BindGroupLayout,
        buffer: GpuVec<T>,
    ) -> BindedBuffer<T>
    where
        T: bytemuck::NoUninit,
    {
        let bind_group = self.create_bind_group(layout, vec![buffer.buffer().as_entire_binding()]);
        BindedBuffer { buffer, bind_group }
    }

    pub fn read_buffer(&self, buffer: &wgpu::Buffer) -> Vec<u8> {
        let data = {
            let buffer_slice = buffer.slice(..);
            let (tx, rx) = oneshot::channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).unwrap();
            });
            self.device.poll(wgpu::Maintain::Wait);
            executor::block_on(rx).unwrap().unwrap();

            let view = buffer_slice.get_mapped_range();
            view.to_vec()
        };
        buffer.unmap();

        data
    }

    pub fn read_texture(&self, texture: &wgpu::Texture) -> Vec<u8> {
        assert!(
            texture.size().width * 4 % 256 == 0,
            "Texture row size must a be multiple of 256"
        );

        let mut encoder = self.device.create_command_encoder(&Default::default());
        let size = texture.size();
        let buffer_length = (size.width * size.height * 4) as wgpu::BufferAddress;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: buffer_length,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::ImageCopyBuffer {
                buffer: &buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(size.width * 4),
                    rows_per_image: None,
                },
            },
            size,
        );
        self.queue.submit(std::iter::once(encoder.finish()));

        self.read_buffer(&buffer)
    }

    pub fn read_texture_to_image(&self, texture: &wgpu::Texture) -> RgbaImage {
        let image_bytes = self.read_texture(texture);
        RgbaImage::from_raw(texture.width(), texture.height(), image_bytes).unwrap()
    }
}

#[derive(Debug)]
pub struct GpuVec<T>
where
    T: bytemuck::NoUninit,
{
    handle: Arc<GpuHandle>,

    inner_buffer: wgpu::Buffer,
    inner_vec: Vec<T>,
}

impl<T> GpuVec<T>
where
    T: bytemuck::NoUninit,
{
    fn create_buffer(
        handle: &GpuHandle,
        usage: wgpu::BufferUsages,
        inner_vec: &Vec<T>,
    ) -> wgpu::Buffer {
        handle
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: unsafe {
                    // SAFETY:
                    // - contents of the buffer beyond the range of inner_vec are allowed to be undefined,
                    // as long as there is no public way to retrieve a slice of a GpuVec's inner_buffer that goes
                    // beyond the range of inner_vec
                    // - we're still only getting a slice up to inner_vec's capacity, which means it's allocated
                    // (and that's good i think)

                    bytemuck::cast_slice(inner_vec.get_unchecked(..inner_vec.capacity()))
                },
                usage: usage | wgpu::BufferUsages::COPY_DST,
            })
    }

    pub fn new(handle_arc: Arc<GpuHandle>, usage: wgpu::BufferUsages, contents: Vec<T>) -> Self {
        assert!(
            mem::size_of::<T>() > 0,
            "Element type must not be zero-sized"
        );

        let inner_buffer = Self::create_buffer(&handle_arc, usage, &contents);
        Self {
            handle: handle_arc,

            inner_buffer,
            inner_vec: contents,
        }
    }

    #[inline]
    pub fn capacity(&self) -> wgpu::BufferAddress {
        self.inner_buffer.size() / mem::size_of::<T>() as wgpu::BufferAddress
    }

    #[inline]
    pub fn len(&self) -> wgpu::BufferAddress {
        self.inner_vec.len() as wgpu::BufferAddress
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner_vec.is_empty()
    }

    #[inline]
    pub fn usage(&self) -> wgpu::BufferUsages {
        self.inner_buffer.usage()
    }

    /// Returns [None] if empty
    pub fn borrow_buffer(&self) -> Option<wgpu::BufferSlice> {
        if self.is_empty() {
            return None;
        }

        Some(
            self.inner_buffer
                .slice(0..(self.inner_vec.len() * mem::size_of::<T>()) as wgpu::BufferAddress),
        )
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.inner_buffer
    }

    fn recreate_buffer(&mut self) {
        self.inner_buffer =
            Self::create_buffer(&self.handle, self.inner_buffer.usage(), &self.inner_vec);
    }

    fn match_vec_capacity(&mut self) {
        if self.capacity() != self.inner_vec.capacity() as wgpu::BufferAddress {
            self.recreate_buffer();
        }
    }

    fn expand_if_needed(&mut self) -> bool {
        if self.capacity() < self.inner_vec.capacity() as wgpu::BufferAddress {
            self.recreate_buffer();
            return true;
        }

        false
    }

    fn apply_inner_change(&mut self, mut range: Range<usize>) {
        range.end = range.end.min(self.inner_vec.len());
        if range.start >= range.end {
            return;
        }

        self.handle.queue.write_buffer(
            &self.inner_buffer,
            (range.start * mem::size_of::<T>()) as wgpu::BufferAddress,
            bytemuck::cast_slice(&self.inner_vec[range]),
        );
    }

    /// Note: This has to create an entirely new buffer, because fuck you
    pub fn change_usage(&mut self, new_usage: wgpu::BufferUsages) {
        if self.inner_buffer.usage() != new_usage {
            self.inner_buffer = Self::create_buffer(&self.handle, new_usage, &self.inner_vec);
        };
    }

    pub fn clear(&mut self) {
        self.inner_vec.clear();
    }

    pub fn extend(&mut self, iter: impl IntoIterator<Item = T>) {
        let old_len = self.inner_vec.len();
        self.inner_vec.extend(iter);

        let difference = self.inner_vec.len() - old_len;
        if difference > 0 && !self.expand_if_needed() {
            self.apply_inner_change((old_len - 1)..self.inner_vec.len());
        };
    }

    pub fn extend_from_slice(&mut self, slice: &[T]) {
        self.extend(slice.iter().copied());
    }

    pub fn push(&mut self, value: T) {
        self.inner_vec.push(value);
        if !self.expand_if_needed() {
            self.apply_inner_change((self.inner_vec.len() - 1)..self.inner_vec.len())
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        self.inner_vec.pop()
    }

    pub fn replace_contents(&mut self, new_contents: Vec<T>) {
        self.inner_vec = new_contents;
        if !self.expand_if_needed() {
            self.apply_inner_change(0..self.inner_vec.len());
        }
    }

    pub fn set(&mut self, index: usize, value: T) {
        self.inner_vec[index] = value;
        self.apply_inner_change(index..self.inner_vec.len());
    }

    pub fn overwrite_from_start_index(&mut self, start_index: usize, new_contents: &[T]) {
        // note: an index of exactly inner_vex.len() is allowed because
        // we're only doing this check to avoid having to fill in gaps
        if start_index > self.inner_vec.len() {
            panic!(
                "Index {} is out of range (max is {})",
                start_index,
                self.inner_vec.len()
            );
        }

        if new_contents.is_empty() {
            return;
        }

        let required_length = start_index + new_contents.len();
        if required_length > self.inner_vec.capacity() {
            self.inner_vec
                .reserve(required_length - self.inner_vec.len())
        }

        for (i, value) in new_contents.iter().copied().enumerate() {
            let index = start_index + i;
            if index >= self.inner_vec.len() {
                self.inner_vec.push(value);
            } else {
                self.inner_vec[index] = value;
            }
        }

        if !self.expand_if_needed() {
            self.apply_inner_change(start_index..self.inner_vec.len());
        }
    }

    pub fn shrink_to_fit(&mut self) {
        self.inner_vec.shrink_to_fit();
        self.match_vec_capacity();
    }

    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.inner_vec.shrink_to(min_capacity);
        self.match_vec_capacity();
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.inner_vec.iter()
    }
}

impl<T> Clone for GpuVec<T>
where
    T: bytemuck::NoUninit,
{
    fn clone(&self) -> Self {
        Self::new(
            Arc::clone(&self.handle),
            self.usage(),
            self.inner_vec.clone(),
        )
    }
}

impl<T> PartialEq for GpuVec<T>
where
    T: bytemuck::NoUninit + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.inner_vec == other.inner_vec
    }
}

#[derive(Debug, Clone)]
pub struct PipelineDescriptor {
    pub name: &'static str,

    pub shader_source: &'static str,

    pub vertex_shader_entry_point: &'static str,
    pub vertex_format: &'static [wgpu::VertexFormat],
    pub instance_format: Option<&'static [wgpu::VertexFormat]>,

    pub fragment_shader_entry_point: &'static str,
    pub target_format: Option<wgpu::TextureFormat>,

    pub bind_groups: &'static [&'static BindGroupFormat],

    pub use_depth: bool,
    pub alpha_to_coverage_enabled: bool,
}

impl Default for PipelineDescriptor {
    fn default() -> Self {
        Self {
            name: "",

            shader_source: "",

            vertex_shader_entry_point: "vert_main",
            vertex_format: &[],
            instance_format: None,

            fragment_shader_entry_point: "frag_main",
            target_format: None,

            bind_groups: &[],

            use_depth: true,
            alpha_to_coverage_enabled: false,
        }
    }
}

fn generate_vertex_attributes(
    formats: &[wgpu::VertexFormat],
    mut shader_location: u32,
) -> (u64, Vec<wgpu::VertexAttribute>) {
    let mut array_stride = 0u64;

    let mut attributes = Vec::with_capacity(formats.len());
    for format in formats {
        attributes.push(wgpu::VertexAttribute {
            format: *format,
            offset: array_stride,
            shader_location,
        });
        array_stride += format.size();
        shader_location += 1;
    }

    (array_stride, attributes)
}

#[derive(Debug)]
pub struct BindedTexture {
    pub texture: Texture,
    pub bind_group: wgpu::BindGroup,
}

#[derive(Debug)]
pub struct BindedBuffer<T>
where
    T: bytemuck::NoUninit,
{
    pub buffer: GpuVec<T>,
    pub bind_group: wgpu::BindGroup,
}

#[derive(Debug)]
pub struct PipelineBuffers<'a, V, I = u8>
where
    V: bytemuck::NoUninit,
    I: bytemuck::NoUninit,
{
    pub vertices: &'a GpuVec<V>,
    pub instances: Option<&'a GpuVec<I>>,
    pub indices: Option<&'a GpuVec<u32>>,
}

impl<'a, V, I> IntoIterator for PipelineBuffers<'a, V, I>
where
    V: bytemuck::NoUninit,
    I: bytemuck::NoUninit,
{
    type Item = Self;

    type IntoIter = std::iter::Once<Self>;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self)
    }
}

#[derive(Debug)]
pub struct Pipeline<V, I = u8>
where
    V: bytemuck::NoUninit,
    I: bytemuck::NoUninit,
{
    handle: Arc<GpuHandle>,
    descriptor: PipelineDescriptor,
    gpu_pipeline: wgpu::RenderPipeline,
    shader_module: wgpu::ShaderModule,

    dummy_vertex_buffer: wgpu::Buffer,
    dummy_instance_buffer: wgpu::Buffer,

    bind_group_layouts: Vec<wgpu::BindGroupLayout>,

    _phantom: PhantomData<(V, I)>,
}

impl<V, I> Pipeline<V, I>
where
    V: bytemuck::NoUninit,
    I: bytemuck::NoUninit,
{
    pub fn new(controller: &GraphicsController, descriptor: PipelineDescriptor) -> Self {
        let handle = controller.handle_arc();

        let shader_module = handle
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(descriptor.name),
                source: wgpu::ShaderSource::Wgsl(descriptor.shader_source.into()),
            });

        let (vertex_stride, vertex_attributes) =
            generate_vertex_attributes(descriptor.vertex_format, 0);
        let (instance_stride, instance_attributes) =
            if let Some(instance_format) = descriptor.instance_format {
                generate_vertex_attributes(instance_format, vertex_attributes.len() as u32)
            } else {
                (0u64, vec![])
            };

        let bind_group_layouts = descriptor
            .bind_groups
            .iter()
            .map(|&format| handle.create_bind_group_layout(format))
            .collect::<Vec<wgpu::BindGroupLayout>>();

        let gpu_pipeline = handle
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(descriptor.name),
                layout: Some(
                    &handle
                        .device
                        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                            label: Some(descriptor.name),
                            bind_group_layouts: &bind_group_layouts
                                .iter()
                                .collect::<Vec<&wgpu::BindGroupLayout>>(),
                            push_constant_ranges: &[],
                        }),
                ),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: descriptor.vertex_shader_entry_point,
                    compilation_options: Default::default(),
                    buffers: &[
                        wgpu::VertexBufferLayout {
                            array_stride: vertex_stride,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &vertex_attributes,
                        },
                        wgpu::VertexBufferLayout {
                            array_stride: instance_stride,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &instance_attributes,
                        },
                    ],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: descriptor.use_depth.then_some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: descriptor.use_depth,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: descriptor.alpha_to_coverage_enabled,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: descriptor.fragment_shader_entry_point,
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: descriptor
                            .target_format
                            .unwrap_or(wgpu::TextureFormat::Rgba8UnormSrgb),
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
            });

        let dummy_vertex_buffer =
            handle
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("'{}' Dummy Vertex Buffer", descriptor.name)),
                    contents: &vec![0u8; vertex_stride as usize],
                    usage: wgpu::BufferUsages::VERTEX,
                });
        let dummy_instance_buffer =
            handle
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("'{}' Dummy Instance Buffer", descriptor.name)),
                    contents: &vec![0u8; instance_stride as usize],
                    usage: wgpu::BufferUsages::VERTEX,
                });

        Self {
            handle,
            descriptor,
            gpu_pipeline,
            shader_module,

            dummy_vertex_buffer,
            dummy_instance_buffer,

            bind_group_layouts,

            _phantom: PhantomData,
        }
    }

    pub fn create_bind_group(
        &self,
        group_layout_index: usize,
        resources: Vec<wgpu::BindingResource>,
    ) -> wgpu::BindGroup {
        self.handle
            .create_bind_group(&self.bind_group_layouts[group_layout_index], resources)
    }

    pub fn binded_texture(&self, group_layout_index: usize, texture: Texture) -> BindedTexture {
        self.handle
            .binded_texture(&self.bind_group_layouts[group_layout_index], texture)
    }

    pub fn binded_buffer<T>(&self, group_layout_index: usize, buffer: GpuVec<T>) -> BindedBuffer<T>
    where
        T: bytemuck::NoUninit,
    {
        self.handle
            .binded_buffer(&self.bind_group_layouts[group_layout_index], buffer)
    }
}

#[derive(Debug)]
pub struct RenderTarget {
    texture: Texture,
    color_cleared: Cell<bool>,
    depth_texture: Option<Texture>,
    depth_cleared: Cell<bool>,
}

impl RenderTarget {
    pub fn new(handle: &GpuHandle, texture: Texture) -> Self {
        Self {
            depth_texture: Some(Texture::create_depth_texture(
                handle,
                texture.inner_texture.width(),
                texture.inner_texture.height(),
            )),
            texture,
            color_cleared: Cell::new(false),
            depth_cleared: Cell::new(false),
        }
    }

    pub fn no_depth(texture: Texture) -> Self {
        Self {
            texture,
            color_cleared: Cell::new(false),
            depth_texture: None,
            depth_cleared: Cell::new(false),
        }
    }

    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    pub fn width(&self) -> u32 {
        self.texture.inner_texture.width()
    }

    pub fn height(&self) -> u32 {
        self.texture.inner_texture.height()
    }

    pub fn frame(&self) -> Vector2<f32> {
        vec2(self.width() as f32, self.height() as f32)
    }

    /// width / height
    pub fn aspect_ratio(&self) -> f32 {
        self.width() as f32 / self.height() as f32
    }

    pub fn depth_texture(&self) -> Option<&Texture> {
        self.depth_texture.as_ref()
    }

    pub fn clear_color(&self) {
        self.color_cleared.set(false);
    }

    pub fn clear_depth(&self) {
        self.depth_cleared.set(false);
    }

    pub fn clear(&self) {
        self.clear_color();
        self.clear_depth();
    }
}

#[derive(Debug)]
pub struct GraphicsController {
    handle: Arc<GpuHandle>,

    window_surface: wgpu::Surface<'static>,
    window_surface_config: wgpu::SurfaceConfiguration,
    window_size: PhysicalSize<u32>,

    present_pipeline: Option<Pipeline<Vertex2D>>,
    present_vertices: GpuVec<Vertex2D>,
    present_indices: GpuVec<u32>,

    render_targets: LinearMap<&'static str, Rc<RenderTarget>>,
}

impl GraphicsController {
    pub fn new(window: Arc<Window>) -> Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let window_surface = instance.create_surface(Arc::clone(&window))?;
        let adapter = futures::executor::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&window_surface),
            },
        ))
        .ok_or(anyhow!("No adapter"))?;

        let (device, queue) = futures::executor::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::CLEAR_TEXTURE,
                required_limits: wgpu::Limits::default(),
            },
            None,
        ))?;

        let window_size = window.inner_size();
        let window_surface_capabilities = window_surface.get_capabilities(&adapter);
        let window_surface_format = window_surface_capabilities
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(window_surface_capabilities.formats[0]);

        let window_surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: window_surface_format,
            width: window_size.width,
            height: window_size.height,
            present_mode: if cfg!(feature = "no_vsync") {
                wgpu::PresentMode::AutoNoVsync
            } else {
                window_surface_capabilities.present_modes[0]
            },
            desired_maximum_frame_latency: 2,
            alpha_mode: window_surface_capabilities.alpha_modes[0],
            view_formats: vec![],
        };
        window_surface.configure(&device, &window_surface_config);

        let handle = Arc::new(GpuHandle { device, queue });

        let present_vertices = GpuVec::new(
            Arc::clone(&handle),
            wgpu::BufferUsages::VERTEX,
            Vertex2D::fill_screen(GuiColor::WHITE, bbox!([0.0, 0.0], [1.0, 1.0])).to_vec(),
        );
        let present_indices = GpuVec::new(
            Arc::clone(&handle),
            wgpu::BufferUsages::INDEX,
            vec![0, 1, 2, 2, 3, 0],
        );

        let mut controller = Self {
            handle,

            window_surface,
            window_surface_config,
            window_size,

            present_pipeline: None,
            present_vertices,
            present_indices,

            render_targets: LinearMap::new(),
        };

        controller.present_pipeline = Some(Pipeline::new(
            &controller,
            PipelineDescriptor {
                name: "Present to Screen",
                shader_source: include_str!("shaders/present.wgsl"),
                vertex_shader_entry_point: "vert_main",
                vertex_format: Vertex2D::VERTEX_FORMAT,
                instance_format: None,
                fragment_shader_entry_point: "frag_main",
                target_format: Some(window_surface_format),
                bind_groups: &[Texture::STANDARD_BIND_GROUP_LAYOUT],
                use_depth: false,
                alpha_to_coverage_enabled: false,
            },
        ));

        Ok(controller)
    }

    pub fn handle(&self) -> &GpuHandle {
        &self.handle
    }

    pub fn handle_arc(&self) -> Arc<GpuHandle> {
        Arc::clone(&self.handle)
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width * new_size.height == 0 {
            return;
        }

        self.window_size = new_size;
        self.window_surface_config.width = new_size.width;
        self.window_surface_config.height = new_size.height;
        self.window_surface
            .configure(&self.handle.device, &self.window_surface_config);
    }

    pub fn window_surface_format(&self) -> wgpu::TextureFormat {
        self.window_surface_config.format
    }

    pub fn present_to_screen(&self, texture: &Texture) -> Result<()> {
        let output = self.window_surface.get_current_texture()?;
        let output_view = output.texture.create_view(&Default::default());

        self.internal_render(
            &output_view,
            None,
            false,
            false,
            self.present_pipeline.as_ref().unwrap(),
            [PipelineBuffers {
                vertices: &self.present_vertices,
                instances: None,
                indices: Some(&self.present_indices),
            }],
            [&self.present_pipeline.as_ref().unwrap().create_bind_group(
                0,
                vec![
                    wgpu::BindingResource::TextureView(&texture.view),
                    wgpu::BindingResource::Sampler(&texture.sampler),
                ],
            )],
        );

        output.present();

        Ok(())
    }

    pub fn render_target(
        &mut self,
        name: &'static str,
        width: u32,
        height: u32,
    ) -> (bool, Rc<RenderTarget>) {
        let recreate = match self.render_targets.get(name) {
            Some(target) => target.width() != width || target.height() != height,
            None => true,
        };

        if recreate {
            self.render_targets.insert(
                name,
                Rc::new(RenderTarget::new(
                    &self.handle,
                    Texture::new(
                        &self.handle,
                        &wgpu::TextureDescriptor {
                            label: Some(name),
                            size: wgpu::Extent3d {
                                width,
                                height,
                                depth_or_array_layers: 1,
                            },
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format: wgpu::TextureFormat::Rgba8UnormSrgb,
                            usage: wgpu::TextureUsages::COPY_DST
                                | wgpu::TextureUsages::COPY_SRC
                                | wgpu::TextureUsages::TEXTURE_BINDING
                                | wgpu::TextureUsages::RENDER_ATTACHMENT,
                            view_formats: &[],
                        },
                        &wgpu::SamplerDescriptor::default(),
                    ),
                )),
            );
        }

        (recreate, Rc::clone(self.render_targets.get(name).unwrap()))
    }

    pub fn window_sized_render_target(&mut self, name: &'static str) -> (bool, Rc<RenderTarget>) {
        self.render_target(name, self.window_size.width, self.window_size.height)
    }

    pub fn vec<T>(&self, contents: Vec<T>, usage: wgpu::BufferUsages) -> GpuVec<T>
    where
        T: bytemuck::NoUninit,
    {
        GpuVec::new(self.handle_arc(), usage, contents)
    }

    pub fn vertex_vec<T>(&self, contents: Vec<T>) -> GpuVec<T>
    where
        T: bytemuck::NoUninit,
    {
        self.vec(contents, wgpu::BufferUsages::VERTEX)
    }

    pub fn index_vec<T>(&self, contents: Vec<T>) -> GpuVec<T>
    where
        T: bytemuck::NoUninit,
    {
        self.vec(contents, wgpu::BufferUsages::INDEX)
    }

    pub fn uniform_vec<T>(&self, contents: Vec<T>) -> GpuVec<T>
    where
        T: bytemuck::NoUninit,
    {
        self.vec(contents, wgpu::BufferUsages::UNIFORM)
    }

    pub fn render<V, I>(
        &self,
        target: &RenderTarget,
        pipeline: &Pipeline<V, I>,
        buffers: impl IntoIterator<Item = PipelineBuffers<V, I>>,
        bind_groups: impl IntoIterator<Item = &wgpu::BindGroup>,
    ) where
        V: bytemuck::NoUninit,
        I: bytemuck::NoUninit,
    {
        let depth_view = target.depth_texture().map(|texture| &texture.view);
        self.internal_render(
            &target.texture().view,
            depth_view,
            !target.color_cleared.get(),
            !target.depth_cleared.get(),
            pipeline,
            buffers,
            bind_groups,
        );
        target.color_cleared.set(true);
        if pipeline.descriptor.use_depth && depth_view.is_some() {
            target.depth_cleared.set(true);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn internal_render<V, I>(
        &self,
        target_view: &wgpu::TextureView,
        depth_view: Option<&wgpu::TextureView>,
        clear_color: bool,
        clear_depth: bool,
        pipeline: &Pipeline<V, I>,
        buffers: impl IntoIterator<Item = PipelineBuffers<V, I>>,
        bind_groups: impl IntoIterator<Item = &wgpu::BindGroup>,
    ) where
        V: bytemuck::NoUninit,
        I: bytemuck::NoUninit,
    {
        let mut encoder = self
            .handle
            .device
            .create_command_encoder(&Default::default());

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(pipeline.descriptor.name),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: if clear_color {
                            wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 0.0,
                            })
                        } else {
                            wgpu::LoadOp::Load
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: if let Some(depth_view) = depth_view {
                    pipeline.descriptor.use_depth.then_some(
                        wgpu::RenderPassDepthStencilAttachment {
                            view: depth_view,
                            depth_ops: Some(wgpu::Operations {
                                load: if clear_depth {
                                    wgpu::LoadOp::Clear(1.0)
                                } else {
                                    wgpu::LoadOp::Load
                                },
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        },
                    )
                } else {
                    None
                },
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            for (i, bind_group) in bind_groups.into_iter().enumerate() {
                render_pass.set_bind_group(i as u32, bind_group, &[]);
            }

            render_pass.set_pipeline(&pipeline.gpu_pipeline);

            'buffer_loop: for PipelineBuffers {
                vertices,
                instances,
                indices,
            } in buffers
            {
                if let Some(vertex_buffer_slice) = vertices.borrow_buffer() {
                    render_pass.set_vertex_buffer(0, vertex_buffer_slice);

                    let index_count = if let Some(indices) = indices {
                        if let Some(index_buffer_slice) = indices.borrow_buffer() {
                            render_pass
                                .set_index_buffer(index_buffer_slice, wgpu::IndexFormat::Uint32);
                            Some(indices.len())
                        } else {
                            continue 'buffer_loop;
                        }
                    } else {
                        None
                    };

                    let instance_count = if let Some(instances) = instances {
                        if let Some(instance_buffer_slice) = instances.borrow_buffer() {
                            render_pass.set_vertex_buffer(1, instance_buffer_slice);

                            instances.len()
                        } else {
                            continue 'buffer_loop;
                        }
                    } else {
                        render_pass.set_vertex_buffer(1, pipeline.dummy_instance_buffer.slice(..));
                        1
                    };

                    if let Some(index_count) = index_count {
                        render_pass.draw_indexed(
                            0..index_count as u32,
                            0,
                            0..instance_count as u32,
                        );
                    } else {
                        render_pass.draw(0..vertices.len() as u32, 0..instance_count as u32);
                    }
                }
            }
        }

        self.handle.queue.submit(std::iter::once(encoder.finish()));
    }
}
