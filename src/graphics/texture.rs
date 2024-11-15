use super::{graphics_controller::GpuHandle, packing::PackedSection};
use crate::shared::bounding_box::{bbox, BBox2};
use derive_more::*;
use image::{DynamicImage, GenericImageView};
use include_dir::include_dir;
use lazy_static::lazy_static;
use std::{collections::BTreeMap, mem};

#[derive(Debug)]
pub struct Texture {
    pub inner_texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

lazy_static! {
    pub static ref SAMPLER_PIXELATED: wgpu::SamplerDescriptor<'static> = wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    };
    pub static ref SAMPLER_LINEAR: wgpu::SamplerDescriptor<'static> = wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    };
    pub static ref SAMPLER_DEPTH: wgpu::SamplerDescriptor<'static> = wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        compare: Some(wgpu::CompareFunction::LessEqual),
        lod_min_clamp: 0.0,
        lod_max_clamp: 100.0,
        ..Default::default()
    };
}

lazy_static! {
    pub static ref TEXTURE_IMAGE: wgpu::TextureDescriptor<'static> = wgpu::TextureDescriptor {
        label: None,
        size: Default::default(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[]
    };
    pub static ref TEXTURE_DEPTH: wgpu::TextureDescriptor<'static> = wgpu::TextureDescriptor {
        label: Some("depth_texture"),
        format: wgpu::TextureFormat::Depth32Float,
        ..*TEXTURE_IMAGE
    };
}

lazy_static! {
    pub static ref TEXTURE_IMAGES: BTreeMap<String, DynamicImage> = {
        const TEXTURE_DIR: include_dir::Dir =
            include_dir!("$CARGO_MANIFEST_DIR/src/graphics/textures");

        fn extract_files<'a>(
            out: &mut Vec<include_dir::File<'a>>,
            entry: include_dir::DirEntry<'a>,
        ) {
            match entry {
                include_dir::DirEntry::Dir(dir) => {
                    for child_entry in dir.entries() {
                        extract_files(out, child_entry.to_owned());
                    }
                }
                include_dir::DirEntry::File(file) => out.push(file),
            }
        }

        let mut files = Vec::<include_dir::File>::new();
        for entry in TEXTURE_DIR.entries() {
            extract_files(&mut files, entry.to_owned());
        }

        let mut images = BTreeMap::new();

        for file in files {
            if let Ok(img) = image::load_from_memory(file.contents()) {
                images.insert(
                    file.path()
                        .file_stem()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                    img,
                );
            }
        }

        images
    };
}

impl Texture {
    pub const STANDARD_BIND_GROUP_LAYOUT: &'static [(wgpu::ShaderStages, wgpu::BindingType)] = &[
        (
            wgpu::ShaderStages::FRAGMENT,
            wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
        ),
        (
            wgpu::ShaderStages::FRAGMENT,
            wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        ),
    ];
    pub const ARRAY_BIND_GROUP_LAYOUT: &'static [(wgpu::ShaderStages, wgpu::BindingType)] = &[
        (
            wgpu::ShaderStages::FRAGMENT,
            wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2Array,
                multisampled: false,
            },
        ),
        (
            wgpu::ShaderStages::FRAGMENT,
            wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        ),
    ];

    pub fn new(
        handle: &GpuHandle,
        texture_descriptor: &wgpu::TextureDescriptor,
        sampler_descriptor: &wgpu::SamplerDescriptor,
    ) -> Self {
        let texture = handle.device.create_texture(texture_descriptor);
        let view = texture.create_view(&Default::default());
        let sampler = handle.device.create_sampler(sampler_descriptor);

        Self {
            inner_texture: texture,
            view,
            sampler,
        }
    }

    pub fn from_image(
        handle: &GpuHandle,
        img: &image::DynamicImage,
        texture_descriptor: &wgpu::TextureDescriptor,
        sampler_descriptor: &wgpu::SamplerDescriptor,
    ) -> Self {
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let modified_texture_descriptor = wgpu::TextureDescriptor {
            size,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: texture_descriptor.usage | wgpu::TextureUsages::COPY_DST,
            ..*texture_descriptor
        };

        let texture = handle.device.create_texture(&modified_texture_descriptor);

        handle.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&Default::default());
        let sampler = handle.device.create_sampler(sampler_descriptor);

        Self {
            inner_texture: texture,
            view,
            sampler,
        }
    }

    pub fn create_depth_texture(handle: &GpuHandle, width: u32, height: u32) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        Self::new(
            handle,
            &wgpu::TextureDescriptor {
                size,
                ..*TEXTURE_DEPTH
            },
            &SAMPLER_DEPTH,
        )
    }

    pub fn clone(&self, handle: &GpuHandle, sampler_descriptor: &wgpu::SamplerDescriptor) -> Self {
        let texture = handle.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: self.inner_texture.size(),
            mip_level_count: self.inner_texture.mip_level_count(),
            sample_count: self.inner_texture.sample_count(),
            dimension: self.inner_texture.dimension(),
            format: self.inner_texture.format(),
            usage: self.inner_texture.usage(),
            view_formats: &[],
        });

        let mut encoder = handle.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_texture(
            self.inner_texture.as_image_copy(),
            texture.as_image_copy(),
            self.inner_texture.size(),
        );
        handle.queue.submit(std::iter::once(encoder.finish()));

        let sampler = handle.device.create_sampler(sampler_descriptor);
        let view = texture.create_view(&Default::default());

        Self {
            inner_texture: texture,
            view,
            sampler,
        }
    }
}

#[derive(Debug, Clone, Copy, From, Into)]
pub struct UVHelper(pub u32, pub u32);

impl UVHelper {
    pub fn bbox(self, corner_0: impl Into<(u32, u32)>, corner_1: impl Into<(u32, u32)>) -> BBox2 {
        let corner_0: (u32, u32) = corner_0.into();
        let corner_1: (u32, u32) = corner_1.into();

        bbox!(
            (
                corner_0.0 as f32 / self.0 as f32,
                corner_0.1 as f32 / self.1 as f32
            ),
            (
                corner_1.0 as f32 / self.0 as f32,
                corner_1.1 as f32 / self.1 as f32
            )
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FourCorners<T> {
    pub top_left: T,
    pub top_right: T,
    pub bottom_left: T,
    pub bottom_right: T,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrientedSection {
    pub section: PackedSection,
    pub flipped: bool,
    pub clockwise_rotations: u8,
}

impl From<PackedSection> for OrientedSection {
    fn from(value: PackedSection) -> Self {
        Self {
            section: value,
            flipped: false,
            clockwise_rotations: 0,
        }
    }
}

impl From<BBox2> for OrientedSection {
    fn from(value: BBox2) -> Self {
        PackedSection::from(value).into()
    }
}

impl OrientedSection {
    pub fn flipped(section: PackedSection) -> Self {
        Self {
            section,
            flipped: true,
            clockwise_rotations: 0,
        }
    }

    pub fn rotated(section: PackedSection, clockwise_rotations: u8) -> Self {
        Self {
            section,
            flipped: false,
            clockwise_rotations,
        }
    }

    pub fn with_flipped(mut self, flipped: bool) -> Self {
        self.flipped = flipped;
        self
    }

    pub fn with_rotations(mut self, clockwise_rotations: u8) -> Self {
        self.clockwise_rotations = clockwise_rotations;
        self
    }

    pub fn uv_corners(self) -> FourCorners<[f32; 2]> {
        let uv = self.section.uv;

        let mut top_left = uv.get_corner([false, false]);
        let mut bottom_right = uv.get_corner([true, true]);
        let mut top_right = uv.get_corner([true, false]);
        let mut bottom_left = uv.get_corner([false, true]);

        if self.flipped {
            mem::swap(&mut top_left, &mut top_right);
            mem::swap(&mut bottom_left, &mut bottom_right);
        }

        for _ in 0..self.clockwise_rotations.rem_euclid(4) {
            let temp_top_left = top_left;

            top_left = bottom_left;
            bottom_left = bottom_right;
            bottom_right = top_right;
            top_right = temp_top_left;
        }

        FourCorners {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        }
    }

    pub fn local_uv(mut self, local_uv: BBox2) -> Self {
        self.section = self.section.local_uv(local_uv);
        self
    }
}
