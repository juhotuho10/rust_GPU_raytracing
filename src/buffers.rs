use wgpu::{util::DeviceExt, BindGroup, BindGroupLayout, Buffer};

fn vec3_pad(x: f32, y: f32, z: f32) -> [f32; 4] {
    [x, y, z, 0.0]
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Params {
    pub width: u32,              // float, aligned to 4 bytes
    pub accumulation_index: u32, // u32, aligned to 4 bytes
    pub _padding: [u8; 8],       // padding to ensure 16-byte alignment
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct RayCamera {
    pub origin: [f32; 4],    // vec4, aligned to 16 bytes
    pub direction: [f32; 4], // vec4, aligned to 16 bytes
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
pub struct SceneMaterial {
    pub albedo: [f32; 3],         // vec3, aligned to 12 bytes
    pub roughness: f32,           // f32, aligned to 4 bytes
    pub emission_color: [f32; 3], // vec3, aligned to 12 bytes
    pub metallic: f32,            // f32, aligned to 4 bytes
    pub emission_power: f32,      // f32, aligned to 4 bytes
    pub _padding: [u8; 12],       // padding to ensure 16-byte alignment
}

pub struct DataBuffers {
    pub output_buffer_size: u64,
    pub accumulation_buffer_size: u64,
    pub input_buffer: Buffer,
    pub output_buffer: Buffer,
    pub params_buffer: Buffer,
    pub staging_buffer: Buffer,
    pub camera_buffer: Buffer,
    pub material_buffer: Buffer,
    pub sphere_buffer: Buffer,
    pub accumulation_buffer: Buffer,
}

impl DataBuffers {
    pub fn new(
        device: &wgpu::Device,
        size: &winit::dpi::PhysicalSize<u32>,
        camera_rays: &[Ray],
        material_array: &[SceneMaterial],
        sphere_array: &[SceneSphere],
        params: &[Params],
    ) -> (DataBuffers, BindGroupLayout, BindGroup) {
        let input_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Input Buffer"),
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

        // Create staging buffer for reading back data
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer"),
            size: output_buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create uniform buffer
        let camera = RayCamera {
            origin: vec3_pad(-7.0, -7.0, 25.),
            direction: vec3_pad(0.0, 0.0, -1.0),
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

        let buffers = DataBuffers {
            output_buffer_size,
            accumulation_buffer_size,
            input_buffer,
            output_buffer,
            params_buffer,
            staging_buffer,
            camera_buffer,
            material_buffer,
            sphere_buffer,
            accumulation_buffer,
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
                    resource: self.input_buffer.as_entire_binding(),
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
            ],
            label: None,
        });

        (bind_group_layout, compute_bind_group)
    }
}
