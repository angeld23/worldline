use crate::{
    graphics::{
        camera::Camera,
        graphics_controller::{
            BindedTexture, GpuHandle, GpuVec, GraphicsController, Pipeline, PipelineBuffers,
            PipelineDescriptor, RenderTarget,
        },
        model::{Model, MODEL_DATA},
        texture::{self, OrientedSection, Texture, TEXTURE_IMAGES},
        vertex::{EntityInstance, Vertex2D, Vertex3D},
    },
    gui::{
        color::GuiColor,
        component::menu::RootComponent,
        element::GuiContext,
        text::{StyledText, TextBackgroundType, TextLabel},
        transform::{GuiTransform, UDim2},
    },
    shared::{
        indexed_container::{IndexedContainer, IndexedVertices},
        input::InputController,
    },
    special::{
        inertial_frame::InertialFrame,
        transform::{lorentz_boost, lorentz_factor},
        universe::{Entity, Universe},
        worldline::{Worldline, PHYS_TIME_STEP},
    },
};
use crate::{
    graphics::{
        camera::CameraUniform,
        graphics_controller::BindedBuffer,
        packing::{PackResult, PackedSection, Packer},
    },
    shared::performance_counter::{PerformanceCounter, PerformanceReport},
};
use anyhow::Result;
use cgmath::{vec2, vec3, vec4, InnerSpace, Matrix4, Vector4};
use linear_map::LinearMap;
use log::{debug, warn};
use obj::{IndexTuple, SimplePolygon};
use rand::Rng;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{
    event::{DeviceEvent, WindowEvent},
    window::Window,
};

use super::player::PlayerController;

#[derive(Debug, Clone, Copy)]
pub enum WinitEvent<'a> {
    Window(&'a WindowEvent),
    Device(&'a DeviceEvent),
}

#[derive(Debug)]
pub struct TextureProvider {
    main_texture: BindedTexture,
    texture_sections: LinearMap<String, PackedSection>,
    reserved_textures: LinearMap<String, wgpu::Texture>,
    packer: Packer,
    handle: Arc<GpuHandle>,
}

impl TextureProvider {
    pub const TEXTURE_SIDE_LENGTH: u32 = 2048;
    pub const PADDING: u32 = 2;

    fn texture_descriptor(layers: u32) -> wgpu::TextureDescriptor<'static> {
        wgpu::TextureDescriptor {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            size: wgpu::Extent3d {
                width: Self::TEXTURE_SIDE_LENGTH,
                height: Self::TEXTURE_SIDE_LENGTH,
                // we need at least 2 layers, otherwise a texture view created with a
                // default descriptor (like in Texture::new) will have a dimension of D2 instead of D2Array
                depth_or_array_layers: layers.max(2),
            },
            ..*texture::TEXTURE_IMAGE
        }
    }

    pub fn new(handle: Arc<GpuHandle>) -> Self {
        Self {
            main_texture: handle.binded_texture(
                &handle.create_bind_group_layout(Texture::ARRAY_BIND_GROUP_LAYOUT),
                Texture::new(
                    &handle,
                    &Self::texture_descriptor(1),
                    &texture::SAMPLER_PIXELATED,
                ),
            ),
            texture_sections: Default::default(),
            reserved_textures: Default::default(),
            packer: Packer::new(
                Self::TEXTURE_SIDE_LENGTH,
                Self::TEXTURE_SIDE_LENGTH,
                Self::PADDING,
            ),
            handle,
        }
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.main_texture.bind_group
    }

    pub fn layer_count(&self) -> u32 {
        self.main_texture
            .texture
            .inner_texture
            .depth_or_array_layers()
    }

    pub fn reserve_slot(&mut self, name: impl Into<String>, width: u32, height: u32) -> bool {
        self.packer.reserve(name, width, height)
    }

    pub fn reserve_texture(
        &mut self,
        name: impl Into<String>,
        texture: wgpu::Texture,
    ) -> Option<wgpu::Texture> {
        let name = name.into();
        if !self
            .packer
            .reserve(&name, texture.width(), texture.height())
        {
            Some(texture)
        } else {
            self.reserved_textures.insert(name, texture);
            None
        }
    }

    pub fn reset_main_texture(&mut self, layers: u32) {
        self.main_texture = self.handle.binded_texture(
            &self
                .handle
                .create_bind_group_layout(Texture::ARRAY_BIND_GROUP_LAYOUT),
            Texture::new(
                &self.handle,
                &Self::texture_descriptor(layers),
                &texture::SAMPLER_PIXELATED,
            ),
        );
    }

    pub fn pack(&mut self) {
        let packer = std::mem::replace(
            &mut self.packer,
            Packer::new(
                Self::TEXTURE_SIDE_LENGTH,
                Self::TEXTURE_SIDE_LENGTH,
                Self::PADDING,
            ),
        );
        let PackResult {
            total_layers,
            sections,
        } = packer.pack();

        self.reset_main_texture(total_layers);
        self.texture_sections = sections;

        for (name, texture) in std::mem::take(&mut self.reserved_textures) {
            self.write_texture(name, &texture);
        }
    }

    pub fn write_texture(&self, name: impl Into<String>, texture: &wgpu::Texture) -> bool {
        let name = name.into();
        if let Some(&section) = self.texture_sections.get(&name) {
            if section.layer_index < self.layer_count() {
                let mut encoder = self
                    .handle
                    .device
                    .create_command_encoder(&Default::default());

                encoder.copy_texture_to_texture(
                    texture.as_image_copy(),
                    wgpu::ImageCopyTexture {
                        texture: &self.main_texture.texture.inner_texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d {
                            x: (section.uv.min()[0] * Self::TEXTURE_SIDE_LENGTH as f32) as u32,
                            y: (section.uv.min()[1] * Self::TEXTURE_SIDE_LENGTH as f32) as u32,
                            z: section.layer_index,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    texture.size(),
                );

                self.handle.queue.submit(std::iter::once(encoder.finish()));

                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn get_packed_section(&self, name: &str) -> PackedSection {
        *self
            .texture_sections
            .get(name)
            .unwrap_or_else(|| self.texture_sections.get("fallback").unwrap())
    }

    pub fn get_section(&self, name: &str) -> OrientedSection {
        self.get_packed_section(name).unoriented()
    }
}

#[derive(Debug)]
struct AppStateGraphics {
    pub texture_provider: TextureProvider,
    pub models: BTreeMap<String, Model>,

    pub generic_quad_indices: GpuVec<u32>,
    pub generic_vertices_2d: GpuVec<Vertex2D>,

    pub pipeline_3d: Pipeline<Vertex3D, EntityInstance>,
    pub instance_buffer: GpuVec<EntityInstance>,
    pub entity_model_instances: BTreeMap<String, Vec<EntityInstance>>,
    pub camera_uniform: BindedBuffer<CameraUniform>,

    pub pipeline_2d: Pipeline<Vertex2D>,
    pub gui_vertices: IndexedVertices<Vertex2D>,
}

#[derive(Debug)]
pub struct AppState {
    pub graphics_controller: GraphicsController,
    pub input_controller: InputController,
    pub gui: RootComponent,
    pub universe: Universe,
    pub player_controller: PlayerController,

    frame_counter: PerformanceCounter,
    last_performance_report: (Instant, Option<PerformanceReport>),

    graphics: AppStateGraphics,
}

impl AppState {
    pub fn new(window: Arc<Window>) -> Result<Self> {
        let graphics_controller = GraphicsController::new(window)?;
        let input_controller = InputController::new();
        let gui = RootComponent::default();

        let generic_quad_indices = graphics_controller.index_vec(vec![0, 1, 2, 2, 3, 0]);
        let generic_vertices_2d = graphics_controller.vertex_vec(vec![]);

        let mut texture_provider = TextureProvider::new(graphics_controller.handle_arc());
        for (name, img) in TEXTURE_IMAGES.iter() {
            let texture = Texture::from_image(
                graphics_controller.handle(),
                img,
                &wgpu::TextureDescriptor {
                    usage: wgpu::TextureUsages::COPY_SRC | texture::TEXTURE_IMAGE.usage,
                    ..*texture::TEXTURE_IMAGE
                },
                &texture::SAMPLER_PIXELATED,
            );

            texture_provider.reserve_texture(name, texture.inner_texture);
        }

        texture_provider.pack();

        let mut models = BTreeMap::new();
        for (name, data) in MODEL_DATA.iter() {
            let texture_section = texture_provider.get_section(name);
            let mut vertices =
                IndexedContainer::with_capacity(data.position.len(), data.position.len());

            for object in data.objects.iter() {
                for group in object.groups.iter() {
                    for SimplePolygon(tuples) in group.polys.iter() {
                        if tuples.len() == 3 {
                            for &IndexTuple(position_index, uv_index, normal_index) in tuples.iter()
                            {
                                let position = data.position[position_index];
                                let uv = data
                                    .texture
                                    .get(uv_index.unwrap_or_default())
                                    .copied()
                                    .unwrap_or([0.0, 0.0]);
                                let normal = data
                                    .normal
                                    .get(normal_index.unwrap_or_default())
                                    .copied()
                                    .unwrap_or([1.0, 0.0, 0.0]);

                                // this kinda sucks because we don't take advantage of vertex indexing
                                // but i don't feel like writing an algorithm to convert the seperately indexed positions,
                                // texture coords, and surface normals into a shared-index container
                                vertices.items.push(Vertex3D {
                                    pos: position,
                                    uv: texture_section.section.local_point(uv.into()).into(),
                                    tex_index: texture_section.section.layer_index,
                                    normal,
                                });
                                vertices.indices.push(vertices.indices.len() as u32);
                            }
                        }
                    }
                }
            }

            models.insert(
                name.to_owned(),
                Model {
                    vertices: IndexedVertices::from_contents(&graphics_controller, vertices),
                },
            );
        }

        // 3D

        let pipeline_3d = Pipeline::new(
            &graphics_controller,
            PipelineDescriptor {
                name: "3D Pipeline",
                shader_source: include_str!("../graphics/shaders/main_3d.wgsl"),
                vertex_shader_entry_point: "vert_main",
                vertex_format: Vertex3D::VERTEX_FORMAT,
                instance_format: Some(EntityInstance::INSTANCE_FORMAT),
                fragment_shader_entry_point: "frag_main",
                target_format: None,
                bind_groups: &[
                    Texture::ARRAY_BIND_GROUP_LAYOUT,
                    &[(
                        wgpu::ShaderStages::VERTEX,
                        wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                    )],
                ],
                use_depth: true,
                alpha_to_coverage_enabled: true,
            },
        );

        let instance_buffer = graphics_controller.vertex_vec(vec![]);
        let entity_model_instances = BTreeMap::new();
        let camera_uniform = pipeline_3d.binded_buffer(
            1,
            graphics_controller.uniform_vec(vec![Camera::default().uniform(1.0)]),
        );

        // 2D

        let pipeline_2d = Pipeline::new(
            &graphics_controller,
            PipelineDescriptor {
                name: "2D Pipeline",
                shader_source: include_str!("../graphics/shaders/main_2d.wgsl"),
                vertex_shader_entry_point: "vert_main",
                vertex_format: Vertex2D::VERTEX_FORMAT,
                instance_format: None,
                fragment_shader_entry_point: "frag_main",
                target_format: None,
                bind_groups: &[Texture::ARRAY_BIND_GROUP_LAYOUT],
                use_depth: false,
                alpha_to_coverage_enabled: false,
            },
        );

        let gui_vertices = IndexedVertices::new(&graphics_controller);

        let graphics = AppStateGraphics {
            texture_provider,
            models,

            generic_quad_indices,
            generic_vertices_2d,

            pipeline_3d,
            instance_buffer,
            entity_model_instances,
            camera_uniform,

            pipeline_2d,
            gui_vertices,
        };

        let mut universe = Universe::default();

        let mut rng = rand::thread_rng();
        let range = 5;
        for x in -range..range {
            for y in -range..range {
                for z in -range..range {
                    universe.insert_entity(Entity {
                        worldline: Worldline::new(InertialFrame {
                            position: vec4(x as f64 * 50.0, y as f64 * 50.0, z as f64 * 50.0, 0.0),
                            ..Default::default()
                        }),
                        model: Some("subdivided_cube".into()),
                        model_matrix: Matrix4::from_scale(5.0),
                        ..Default::default()
                    });
                }
            }
        }
        // for _ in 0..500 {
        //     universe.insert_entity(Entity {
        //         worldline: Worldline::new(InertialFrame {
        //             position: vec4(
        //                 rng.gen_range(-500.0..500.0),
        //                 rng.gen_range(-500.0..500.0),
        //                 rng.gen_range(-500.0..500.0),
        //                 0.0,
        //             ),
        //             ..Default::default()
        //         }),
        //         model: Some("subdivided_cube".into()),
        //         model_matrix: Matrix4::from_scale(5.0),
        //         ..Default::default()
        //     });
        // }

        let player_controller = PlayerController::default();

        Ok(Self {
            graphics_controller,
            input_controller,
            gui,
            universe,
            player_controller,

            frame_counter: PerformanceCounter::new(),
            last_performance_report: (Instant::now(), None),

            graphics,
        })
    }

    pub fn phys_tick(&mut self) {
        self.universe.step(PHYS_TIME_STEP);
    }

    pub fn window_focus_changed(&mut self, is_focused: bool) {}

    pub fn update_camera_uniform(&mut self, camera: Camera, aspect_ratio: f32) {
        self.graphics
            .camera_uniform
            .buffer
            .replace_contents(vec![camera.uniform(aspect_ratio)]);
    }

    pub fn render_simple_sky(&mut self, target: &RenderTarget) {
        let color = GuiColor {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        };

        self.graphics.generic_vertices_2d.replace_contents(
            Vertex2D::fill_screen(color, self.graphics.texture_provider.get_section("white"))
                .to_vec(),
        );

        self.graphics_controller.render(
            target,
            &self.graphics.pipeline_2d,
            PipelineBuffers {
                vertices: &self.graphics.generic_vertices_2d,
                instances: None,
                indices: Some(&self.graphics.generic_quad_indices),
            },
            [self.graphics.texture_provider.bind_group()],
        );
    }

    pub fn update_entity_model_instances(&mut self) {
        for (_, list) in self.graphics.entity_model_instances.iter_mut() {
            list.clear();
        }

        let user_entity = self.universe.get_user_entity();
        let user_event = user_entity.worldline.get_event_at_time(self.universe.time);
        let user_frame = user_event.frame;

        let new_model_instances: Vec<(String, EntityInstance)> = self
            .universe
            .entities
            .par_iter()
            .filter_map(|(_, entity)| {
                let model_name = entity.model.as_ref()?;
                if !self.graphics.models.contains_key(model_name) {
                    warn!("Model '{}' does not exist", model_name);
                    return None;
                }

                // lightspeed delay
                let event = {
                    // use newton's method for finding the event whose delay matches the expected
                    // delay given its distance
                    let mut estimated_event =
                        entity.worldline.get_event_at_time(self.universe.time);
                    let mut prev_offset: Option<f64> = None;
                    let mut prev_change: Option<f64> = None;
                    for _ in 0..30 {
                        let relative_frame = estimated_event.frame.relative_to(user_frame);
                        let relative_gamma = lorentz_factor(relative_frame.velocity);
                        let travel_time = (estimated_event.frame.position - user_frame.position)
                            .truncate()
                            .magnitude();
                        let timeline_delay = self.universe.time - estimated_event.frame.position.w;
                        let offset = timeline_delay - travel_time;

                        let change = if let (Some(prev_offset), Some(prev_change)) =
                            (prev_offset, prev_change)
                        {
                            let derivative = (prev_offset - offset) / prev_change;

                            offset / derivative
                        } else {
                            offset / relative_gamma
                        };

                        prev_offset = Some(offset);
                        prev_change = Some(change);

                        if offset.abs() < 0.001 {
                            break;
                        }

                        estimated_event = entity
                            .worldline
                            .get_event_at_time(estimated_event.frame.position.w + change);
                    }
                    estimated_event
                };

                let relative_frame = event.frame.relative_to(user_frame);
                let relative_boost = lorentz_boost(relative_frame.velocity);

                let contraction = vec3(
                    1.0 / (relative_boost * Vector4::unit_x()).x as f32,
                    1.0 / (relative_boost * Vector4::unit_y()).y as f32,
                    1.0 / (relative_boost * Vector4::unit_z()).z as f32,
                );

                let contraction_matrix =
                    Matrix4::from_nonuniform_scale(contraction.x, contraction.y, contraction.z);
                let model_matrix =
                    Matrix4::from_translation(relative_frame.position.truncate().map(|v| v as f32))
                        * contraction_matrix
                        * entity.model_matrix;

                Some((
                    model_name.to_owned(),
                    EntityInstance {
                        model_matrix: model_matrix.into(),
                        velocity: relative_frame.velocity.map(|v| v as f32).into(),
                        color: entity.model_color.into(),
                    },
                ))
            })
            .collect();

        for (model_name, instance) in new_model_instances {
            self.graphics
                .entity_model_instances
                .entry(model_name)
                .or_default()
                .push(instance);
        }
    }

    pub fn render_entities(&mut self, target: &RenderTarget) {
        for (model_name, instances) in self.graphics.entity_model_instances.iter() {
            if let Some(model) = self.graphics.models.get(model_name) {
                self.graphics
                    .instance_buffer
                    .replace_contents(instances.clone());
                self.graphics_controller.render(
                    target,
                    &self.graphics.pipeline_3d,
                    PipelineBuffers {
                        vertices: &model.vertices.vertices,
                        instances: Some(&self.graphics.instance_buffer),
                        indices: Some(&model.vertices.indices),
                    },
                    [
                        self.graphics.texture_provider.bind_group(),
                        &self.graphics.camera_uniform.bind_group,
                    ],
                );
            } else {
                warn!("Model '{}' does not exist", model_name);
            }
        }
    }

    pub fn render(&mut self, delta: f64) {
        self.player_controller
            .update(&mut self.universe, &mut self.input_controller, delta);

        let (_, window_target) = self
            .graphics_controller
            .window_sized_render_target("render");
        window_target.clear();

        self.render_simple_sky(&window_target);

        // 3d rendering
        {
            self.update_camera_uniform(self.player_controller.camera, window_target.aspect_ratio());
            self.update_entity_model_instances();
            self.render_entities(&window_target);
        }

        // 2d rendering
        {
            let mut gui_builder = GuiContext::new(
                window_target.frame(),
                &self.graphics.texture_provider,
                &mut self.input_controller,
            )
            .builder();

            self.gui.render(&mut gui_builder);

            self.frame_counter.tick();

            let report_string = if let Some(PerformanceReport {
                mean,
                slowest,
                fastest,
                ..
            }) = self.last_performance_report.1
            {
                let mean_ms = mean.as_micros() as f64 / 1000.0;
                let slowest_ms = slowest.as_micros() as f64 / 1000.0;
                let fastest_ms = fastest.as_micros() as f64 / 1000.0;

                let mean_fps = (1.0 / mean.as_secs_f64()) as u32;
                let slowest_fps = (1.0 / slowest.as_secs_f64()) as u32;
                let fastest_fps = (1.0 / fastest.as_secs_f64()) as u32;

                format!("§b{mean_ms}ms/{mean_fps}fps §r(§a↑{fastest_ms}ms/{fastest_fps}fps§r | §c↓{slowest_ms}ms/{slowest_fps}fps§r)")
            } else {
                "...".to_owned()
            };

            if self.last_performance_report.0.elapsed() > Duration::from_millis(1000) {
                self.last_performance_report.1 = self.frame_counter.flush();
                self.last_performance_report.0 = Instant::now();

                debug!("{}", StyledText::from_format_string(&report_string));
            }

            let user_event = self.universe.user_event_now();
            let pos = user_event.frame.position.truncate();
            let vel = user_event.frame.velocity;
            let debug_text = format!(
                "Displacement: {:.3}, {:.3}, {:.3} ({:.3}cs from origin)\nVelocity: {:.3}c ({:.3}, {:.3}, {:.3})\nLorentz factor: {:.3}\n{}",
                pos.x, pos.y, pos.z, pos.magnitude(), vel.magnitude(), vel.x, vel.y, vel.z, lorentz_factor(vel), report_string,);

            gui_builder.element(TextLabel {
                transform: GuiTransform {
                    size: UDim2::from_scale(1.0, 1.0),
                    ..Default::default()
                },
                text: StyledText::from_format_string(&debug_text),
                char_pixel_height: 16.0,
                text_alignment: vec2(0.0, 0.0),
                background_color: GuiColor::BLACK.with_alpha(0.75),
                background_type: TextBackgroundType::BoundingBoxPerLine,
            });

            let finished_vertices = gui_builder.finish();

            self.graphics
                .gui_vertices
                .replace_contents(finished_vertices);
            self.graphics_controller.render(
                &window_target,
                &self.graphics.pipeline_2d,
                self.graphics.gui_vertices.as_pipeline_buffers(),
                [self.graphics.texture_provider.bind_group()],
            );
        }

        let _ = self
            .graphics_controller
            .present_to_screen(window_target.texture());
    }

    pub fn winit_event(&mut self, event: WinitEvent) {
        self.input_controller.winit_event(event);
    }
}
