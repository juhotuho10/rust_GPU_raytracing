use glam::Vec3A;

use wgpu::{util::DeviceExt, BindGroup, BindGroupLayout, Buffer, Device, Queue, Texture};

use super::image_texture::*;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Params {
    pub screen_width: u32,       // float, aligned to 4 bytes
    pub accumulation_index: u32, // u32, aligned to 4 bytes
    pub accumulate: u32,         // u32, aligned to 4 bytes
    pub sphere_count: u32,       // u32, aligned to 4 bytes
    pub object_count: u32,       // u32, aligned to 4 bytes
    pub compute_per_frame: u32,  // u32, aligned to 4 bytes
    pub texture_width: u32,      // u32, aligned to 4 bytes
    pub texture_height: u32,     // u32, aligned to 4 bytes
    pub textue_count: u32,       // u32, aligned to 4 bytes
    pub env_map_width: u32,      // u32, aligned to 4 bytes
    pub env_map_height: u32,     // u32, aligned to 4 bytes
    pub _padding: [u8; 4],       // padding to ensure 16-byte alignment
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RayCamera {
    pub origin: [f32; 3],  // vec3, aligned to 12 bytes
    pub _padding: [u8; 4], // padding to ensure 16-byte alignment
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Ray {
    pub direction: [f32; 3], // vec3, aligned to 12 bytes
    pub _padding: [u8; 4],   // padding to ensure 16-byte alignment
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SceneSphere {
    pub position: [f32; 3],  // vec3, aligned to 12 bytes
    pub radius: f32,         // f32, aligned to 4 bytes
    pub material_index: u32, // u32, aligned to 4 bytes
    pub _padding: [u8; 12],  // padding to ensure 16-byte alignment
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SceneTriangle {
    a: [f32; 3],              //
    _padding: [u8; 4],        // padding to ensure 16-byte alignment
    edge_ab: [f32; 3],        // vec3, aligned to 12 bytes
    _padding2: [u8; 4],       // padding to ensure 16-byte alignment
    edge_ac: [f32; 3],        // vec3, aligned to 12 bytes
    _padding3: [u8; 4],       // padding to ensure 16-byte alignment
    calc_normal: [f32; 3],    // vec3, aligned to 12 bytes
    _padding4: [u8; 4],       // padding to ensure 16-byte alignment
    face_normal: [f32; 3],    // vec3, aligned to 12 bytes
    _padding5: [u8; 4],       // padding to ensure 16-byte alignment
    pub min_bounds: [f32; 3], // vec3, aligned to 12 bytes
    _padding6: [u8; 4],       // padding to ensure 16-byte alignment
    pub max_bounds: [f32; 3], // vec3, aligned to 12 bytes
    _padding7: [u8; 4],       // padding to ensure 16-byte alignment
}

impl SceneTriangle {
    pub fn new(a: Vec3A, b: Vec3A, c: Vec3A) -> SceneTriangle {
        // precalculations to save on compute

        let edge_ab = b - a;
        let edge_ac = c - a;

        let calc_normal = edge_ab.cross(edge_ac);
        let face_normal = calc_normal.normalize();

        let min_bounds = a.min(b).min(c);
        let max_bounds = a.max(b).max(c);

        SceneTriangle {
            a: a.into(),                     // vec3, aligned to 12 bytes
            _padding: [0; 4],                // padding to ensure 16-byte alignment
            edge_ab: edge_ab.into(),         // vec3, aligned to 12 bytes
            _padding2: [0; 4],               // padding to ensure 16-byte alignment
            edge_ac: edge_ac.into(),         // vec3, aligned to 12 bytes
            _padding3: [0; 4],               // padding to ensure 16-byte alignment
            calc_normal: calc_normal.into(), // vec3, aligned to 12 bytes
            _padding4: [0; 4],               // padding to ensure 16-byte alignment
            face_normal: face_normal.into(), // vec3, aligned to 12 bytes
            _padding5: [0; 4],               // padding to ensure 16-byte alignment
            min_bounds: min_bounds.into(),   // vec3, aligned to 12 bytes
            _padding6: [0; 4],               // padding to ensure 16-byte alignment
            max_bounds: max_bounds.into(),   // vec3, aligned to 12 bytes
            _padding7: [0; 4],               // padding to ensure 16-byte alignment
                                             // vec3, aligned to 12 bytes
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SceneMaterial {
    pub texture_index: u32,    // vec3, aligned to 12 bytes
    pub roughness: f32,        // f32, aligned to 4 bytes
    pub emission_power: f32,   // f32, aligned to 4 bytes
    pub specular: f32,         // f32, aligned to 4 bytes
    pub specular_scatter: f32, // f32, aligned to 4 bytes
    pub glass: f32,            // f32, aligned to 4 bytes
    pub refraction_index: f32, // f32, aligned to 4 bytes
    pub _padding: [u8; 4],     // padding to ensure 16-byte alignment
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ObjectInfo {
    pub min_bounds: [f32; 3],        // vec3, aligned to 12 bytes
    pub first_sub_object_index: u32, // f32, aligned to 4 bytes
    pub max_bounds: [f32; 3],        // vec3, aligned to 12 bytes
    pub sub_object_count: u32,       // f32, aligned to 4 bytes
    pub material_index: u32,         // f32, aligned to 4 bytes
    pub _padding: [u8; 12],          // padding to ensure 16-byte alignment
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SubObjectInfo {
    pub min_bounds: [f32; 3],      // vec3, aligned to 12 bytes
    pub first_triangle_index: u32, // f32, aligned to 4 bytes
    pub max_bounds: [f32; 3],      // vec3, aligned to 12 bytes
    pub triangle_count: u32,       // f32, aligned to 4 bytes
}

macro_rules! bind_group_entry {
    ($binding:expr, $resource:expr) => {
        wgpu::BindGroupEntry {
            binding: $binding,
            resource: $resource.as_entire_binding(),
        }
    };
}

pub struct DataBuffers {
    pub output_buffer_size: u64,
    pub accumulation_buffer_size: u64,
    pub ray_buffer: Buffer,
    pub output_buffer: Buffer,
    pub params_buffer: Buffer,
    pub camera_buffer: Buffer,
    pub material_buffer: Buffer,
    pub sphere_buffer: Buffer,
    pub accumulation_buffer: Buffer,
    pub triangle_buffer: Buffer,
    pub object_buffer: Buffer,
    pub sub_object_buffer: Buffer,
    pub image_textures: Texture,
    pub environment_map: Texture,
}

#[allow(clippy::too_many_arguments)]
impl DataBuffers {
    pub fn new(
        device: &wgpu::Device,
        size: &winit::dpi::PhysicalSize<u32>,
        camera: RayCamera,
        camera_rays: &[Ray],
        material_array: &[SceneMaterial],
        sphere_array: &[SceneSphere],
        triangle_array: &[SceneTriangle],
        object_array: &[ObjectInfo],
        sub_object_array: &[SubObjectInfo],
        params: &[Params],
    ) -> (DataBuffers, BindGroupLayout, BindGroup) {
        let ray_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ray Buffer"),
            contents: bytemuck::cast_slice(camera_rays),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // 4 bytes of u8 per pixel, RGBA
        let output_buffer_size = (size.width * size.height * std::mem::size_of::<[u8; 4]>() as u32)
            as wgpu::BufferAddress;

        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: output_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Params Buffer"),
            contents: bytemuck::cast_slice(params),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let material_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Material Buffer"),
            contents: bytemuck::cast_slice(material_array),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let sphere_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sphere Buffer"),
            contents: bytemuck::cast_slice(sphere_array),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // 4 bytes of RGBA f32 per pixel
        let accumulation_buffer_size =
            (size.width * size.height * std::mem::size_of::<[f32; 4]>() as u32)
                as wgpu::BufferAddress;

        let accumulation_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Accumulation Buffer"),
            size: accumulation_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let triangle_buffer: Buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Triangle Buffer"),
                contents: bytemuck::cast_slice(triangle_array),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let object_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Object Buffer"),
            contents: bytemuck::cast_slice(object_array),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let texture_size = wgpu::Extent3d {
            width: params[0].texture_width,
            height: params[0].texture_height,
            depth_or_array_layers: params[0].textue_count,
        };

        let image_textures = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Texture Array"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let sub_object_buffer: Buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Sub Object Buffer"),
                contents: bytemuck::cast_slice(sub_object_array),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let env_map_size = wgpu::Extent3d {
            width: params[0].env_map_width,
            height: params[0].env_map_height,
            depth_or_array_layers: 1,
        };

        let environment_map = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Environment map"),
            size: env_map_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let buffers = DataBuffers {
            output_buffer_size,
            accumulation_buffer_size,
            ray_buffer,
            output_buffer,
            params_buffer,
            camera_buffer,
            material_buffer,
            sphere_buffer,
            accumulation_buffer,
            triangle_buffer,
            object_buffer,
            sub_object_buffer,
            image_textures,
            environment_map,
        };

        let (bind_group_layout, compute_bind_group) = buffers.create_compute_bindgroup(device);

        (buffers, bind_group_layout, compute_bind_group)
    }

    fn create_compute_bindgroup(
        &self,
        device: &wgpu::Device,
    ) -> (wgpu::BindGroupLayout, BindGroup) {
        let params_bind = 0;
        let ray_directions_bind = 1;
        let pixel_colors_bind = 2;
        let camera_bind = 3;
        let material_bind = 4;
        let sphere_bind = 5;
        let accumulation_bind = 6;
        let triangle_bind = 7;
        let object_bind = 8;
        let texture_bind = 9;
        let sub_object_bind = 10;
        let env_map_bind = 11;

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: params_bind,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: ray_directions_bind,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: pixel_colors_bind,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: camera_bind,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: material_bind,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: sphere_bind,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: accumulation_bind,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: triangle_bind,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: object_bind,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: texture_bind,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: sub_object_bind,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: env_map_bind,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
            label: None,
        });

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                bind_group_entry!(params_bind, self.params_buffer),
                bind_group_entry!(ray_directions_bind, self.ray_buffer),
                bind_group_entry!(pixel_colors_bind, self.output_buffer),
                bind_group_entry!(camera_bind, self.camera_buffer),
                bind_group_entry!(material_bind, self.material_buffer),
                bind_group_entry!(sphere_bind, self.sphere_buffer),
                bind_group_entry!(accumulation_bind, self.accumulation_buffer),
                bind_group_entry!(triangle_bind, self.triangle_buffer),
                bind_group_entry!(object_bind, self.object_buffer),
                wgpu::BindGroupEntry {
                    binding: texture_bind,
                    resource: wgpu::BindingResource::TextureView(
                        &self
                            .image_textures
                            .create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                bind_group_entry!(sub_object_bind, self.sub_object_buffer),
                wgpu::BindGroupEntry {
                    binding: env_map_bind,
                    resource: wgpu::BindingResource::TextureView(
                        &self
                            .environment_map
                            .create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
            ],
            label: None,
        });

        (bind_group_layout, compute_bind_group)
    }

    pub fn update_texture_buffer(
        &self,
        textures: &[ImageTexture],
        queue: &Queue,
        texture_width: u32,
        texture_height: u32,
    ) {
        for (i, texture) in textures.iter().enumerate() {
            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &self.image_textures,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: i as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &texture.image_buffer,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * texture_width), // 4x u8 per pixel
                    rows_per_image: Some(texture_height),
                },
                wgpu::Extent3d {
                    width: texture_width,
                    height: texture_height,
                    depth_or_array_layers: 1,
                },
            );
        }
    }

    pub fn update_environment_map_buffer(
        &self,
        env_map_texture: &ImageTexture,
        queue: &Queue,
        texture_width: u32,
        texture_height: u32,
    ) {
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.environment_map,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            &env_map_texture.image_buffer,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * texture_width), // 4x u8 per pixel
                rows_per_image: Some(texture_height),
            },
            wgpu::Extent3d {
                width: texture_width,
                height: texture_height,
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn update_ray_directions(&self, queue: &Queue, new_rays: &[Ray]) {
        queue.write_buffer(&self.ray_buffer, 0, bytemuck::cast_slice(new_rays));
    }

    pub fn reset_accumulation(&mut self, device: &Device, queue: &Queue, params: &[Params]) {
        let mut buffer_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Buffer Encoder"),
        });

        buffer_encoder.clear_buffer(&self.accumulation_buffer, 0, None);

        queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(params));

        queue.submit(Some(buffer_encoder.finish()));
    }

    pub fn update_accumulation(&self, queue: &Queue, params: &[Params]) {
        queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(params));
    }

    pub fn update_spheres(&self, queue: &Queue, new_spheres: &[SceneSphere]) {
        queue.write_buffer(&self.sphere_buffer, 0, bytemuck::cast_slice(new_spheres));
    }

    pub fn update_triangles(&self, queue: &Queue, new_triangles: &[SceneTriangle]) {
        queue.write_buffer(
            &self.triangle_buffer,
            0,
            bytemuck::cast_slice(new_triangles),
        );
    }

    pub fn update_object_info(&self, queue: &Queue, new_object_info: &[ObjectInfo]) {
        queue.write_buffer(
            &self.object_buffer,
            0,
            bytemuck::cast_slice(new_object_info),
        );
    }

    pub fn update_sub_object_info(&self, queue: &Queue, sub_object_array: &[SubObjectInfo]) {
        queue.write_buffer(
            &self.sub_object_buffer,
            0,
            bytemuck::cast_slice(sub_object_array),
        );
    }

    pub fn update_materials(&self, queue: &Queue, new_materials: &[SceneMaterial]) {
        queue.write_buffer(
            &self.material_buffer,
            0,
            bytemuck::cast_slice(new_materials),
        );
    }
}
