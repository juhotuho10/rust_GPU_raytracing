use glam::Vec3A;
use wgpu::{util::DeviceExt, BindGroup, BindGroupLayout, Buffer, Device, Queue};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Params {
    pub sky_color: [f32; 3],     // vec3, aligned to 12 bytes
    pub width: u32,              // float, aligned to 4 bytes
    pub accumulation_index: u32, // u32, aligned to 4 bytes
    pub accumulate: u32,         // u32, aligned to 4 bytes
    pub sphere_count: u32,       // u32, aligned to 4 bytes
    pub triangle_count: u32,     // u32, aligned to 4 bytes
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
    a: [f32; 3],           //
    material_index: u32,   // u32, aligned to 4 bytes
    edge_ab: [f32; 3],     // vec3, aligned to 12 bytes
    _padding: [u8; 4],     // padding to ensure 16-byte alignment
    edge_ac: [f32; 3],     // vec3, aligned to 12 bytes
    _padding2: [u8; 4],    // padding to ensure 16-byte alignment
    calc_normal: [f32; 3], // vec3, aligned to 12 bytes
    _padding3: [u8; 4],    // padding to ensure 16-byte alignment
    face_normal: [f32; 3], // vec3, aligned to 12 bytes
    _padding4: [u8; 4],    // padding to ensure 16-byte alignment
}

impl SceneTriangle {
    pub fn new(material_index: u32, a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> SceneTriangle {
        // precalculations to save on compute
        let a_vec: Vec3A = a.into();
        let b_vec: Vec3A = b.into();
        let c_vec: Vec3A = c.into();

        let edge_ab = b_vec - a_vec;
        let edge_ac = c_vec - a_vec;

        let calc_normal = edge_ab.cross(edge_ac);
        let face_normal = calc_normal.normalize();

        SceneTriangle {
            a,                               // vec3, aligned to 12 bytes
            material_index,                  // u32, aligned to 4 bytes
            edge_ab: edge_ab.into(),         // vec3, aligned to 12 bytes
            _padding: [0; 4],                // padding to ensure 16-byte alignment
            edge_ac: edge_ac.into(),         // vec3, aligned to 12 bytes
            _padding2: [0; 4],               // padding to ensure 16-byte alignment
            calc_normal: calc_normal.into(), // vec3, aligned to 12 bytes
            _padding3: [0; 4],               // padding to ensure 16-byte alignment
            face_normal: face_normal.into(), // vec3, aligned to 12 bytes
            _padding4: [0; 4],               // padding to ensure 16-byte alignment
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SceneMaterial {
    pub albedo: [f32; 3],         // vec3, aligned to 12 bytes
    pub roughness: f32,           // f32, aligned to 4 bytes
    pub emission_color: [f32; 3], // vec3, aligned to 12 bytes
    pub emission_power: f32,      // f32, aligned to 4 bytes
    pub metallic: f32,            // f32, aligned to 4 bytes
    pub specular_scatter: f32,    // f32, aligned to 4 bytes
    pub reflectivity: f32,        // f32, aligned to 4 bytes
    pub _padding: [u8; 4],        // padding to ensure 16-byte alignment
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ObjectInfo {
    pub min_bounds: [f32; 3],      // vec3, aligned to 12 bytes
    pub first_triangle_index: u32, // f32, aligned to 4 bytes
    pub max_bounds: [f32; 3],      // vec3, aligned to 12 bytes
    pub triangle_count: u32,       // f32, aligned to 4 bytes
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
}

#[allow(clippy::too_many_arguments)]
impl DataBuffers {
    pub fn new(
        device: &wgpu::Device,
        size: &winit::dpi::PhysicalSize<u32>,
        camera_rays: &[Ray],
        material_array: &[SceneMaterial],
        sphere_array: &[SceneSphere],
        triangle_array: &[SceneTriangle],
        object_array: &[ObjectInfo],
        params: &[Params],
    ) -> (DataBuffers, BindGroupLayout, BindGroup) {
        let ray_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ray Buffer"),
            contents: bytemuck::cast_slice(camera_rays),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // 4 bytes of u8 per pixel, RGBA
        let output_buffer_size = (size.width * size.height * 4 * std::mem::size_of::<u8>() as u32)
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

        // Create uniform buffer
        let camera = RayCamera {
            origin: [-7.0, -7.0, 25.],
            _padding: [0; 4],
        };

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

        // 3 bytes of f32 per pixel
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
            ],
            label: None,
        });

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: params_bind,
                    resource: self.params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: ray_directions_bind,
                    resource: self.ray_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: pixel_colors_bind,
                    resource: self.output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: camera_bind,
                    resource: self.camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: material_bind,
                    resource: self.material_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: sphere_bind,
                    resource: self.sphere_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: accumulation_bind,
                    resource: self.accumulation_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: triangle_bind,
                    resource: self.triangle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: object_bind,
                    resource: self.object_buffer.as_entire_binding(),
                },
            ],
            label: None,
        });

        (bind_group_layout, compute_bind_group)
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

    pub fn update_materials(&self, queue: &Queue, new_materials: &[SceneMaterial]) {
        queue.write_buffer(
            &self.material_buffer,
            0,
            bytemuck::cast_slice(new_materials),
        );
    }
}
