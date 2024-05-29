use crate::buffers::{ObjectInfo, Params, RayCamera, SceneMaterial, SceneSphere, SceneTriangle};

use crate::triangle_object::SceneObject;

use super::camera::Camera;

use super::buffers;

use egui::Context;

use wgpu::{BindGroup, BindGroupLayout, CommandEncoder, Device, Queue, Texture};

use image::GenericImageView;

#[derive(Debug, Clone)]
pub struct RenderScene {
    pub spheres: Vec<SceneSphere>,
    pub materials: Vec<SceneMaterial>,
    pub objects: Vec<SceneObject>,
    pub sky_color: [f32; 3],
}

pub struct Renderer<'a> {
    pub camera: Camera,
    pub scene: RenderScene,
    pub device: &'a Device,
    pub queue: &'a Queue,
    pub accumulate: bool,
    pub object_index: usize,
    pub sphere_index: usize,
    pub compute_per_frame: u32,
    accumulation_index: u32,
    buffers: buffers::DataBuffers,
    pub texure_array: Texture,
}

impl Renderer<'_> {
    pub fn new<'a>(
        camera: Camera,
        scene: RenderScene,
        device: &'a Device,
        queue: &'a Queue,
        size: winit::dpi::PhysicalSize<u32>,
        params: Params,
    ) -> (Renderer<'a>, BindGroupLayout, BindGroup) {
        let camera_rays = camera.recalculate_ray_directions();
        let accumulate = params.accumulate == 1;

        let (object_info_vec, triangles) = get_triangle_data(&scene);

        let ray_camera: RayCamera = RayCamera {
            origin: camera.position.into(),
            _padding: [0; 4],
        };

        let texture_array = make_texture_array(device, queue);

        let (buffers, bind_group_layout, compute_bind_group) = buffers::DataBuffers::new(
            device,
            &size,
            ray_camera,
            &camera_rays,
            &scene.materials,
            &scene.spheres,
            &triangles,
            &object_info_vec,
            &[params],
            &texture_array,
        );

        let renderer = Renderer {
            camera,
            scene,
            device,
            queue,
            accumulate,
            object_index: 0,
            sphere_index: 0,
            compute_per_frame: params.compute_per_frame,
            accumulation_index: 1,
            buffers,
            texure_array: texture_array,
        };

        (renderer, bind_group_layout, compute_bind_group)
    }

    pub fn on_resize(&mut self, size: &winit::dpi::PhysicalSize<u32>) {
        self.camera.on_resize(size.width, size.height);

        self.reset_accumulation()
    }

    pub fn on_update(&mut self, mouse_delta: egui::Vec2, egui_context: &Context) {
        let moved = self.camera.on_update(mouse_delta, egui_context);

        if moved {
            self.reset_accumulation();
            let new_rays = self.camera.recalculate_ray_directions();

            let new_camera = RayCamera {
                origin: self.camera.position.into(),
                _padding: [0; 4],
            };

            self.queue.write_buffer(
                &self.buffers.camera_buffer,
                0,
                bytemuck::cast_slice(&[new_camera]),
            );

            self.buffers.update_ray_directions(self.queue, &new_rays);
        };
    }

    pub fn reset_accumulation(&mut self) {
        self.accumulation_index = 1;

        let params = Params {
            sky_color: self.scene.sky_color,
            width: self.camera.viewport_width,
            accumulation_index: self.accumulation_index,
            accumulate: self.accumulate as u32,
            sphere_count: self.scene.spheres.len() as u32,
            object_count: self.scene.objects.len() as u32,
            compute_per_frame: self.compute_per_frame,
            _padding: [0; 12],
        };

        self.buffers
            .reset_accumulation(self.device, self.queue, &[params]);
    }

    pub fn update_scene(&mut self) {
        self.reset_accumulation();

        let new_spheres = &self.scene.spheres;
        self.buffers.update_spheres(self.queue, new_spheres);

        for object in &mut self.scene.objects {
            object.update_triangles();
        }
        let (new_object_info, new_triangles) = get_triangle_data(&self.scene);

        self.buffers.update_triangles(self.queue, &new_triangles);

        self.buffers
            .update_object_info(self.queue, &new_object_info);

        let new_materials = &self.scene.materials;
        self.buffers.update_materials(self.queue, new_materials);
    }

    pub fn compute_frame(
        &mut self,
        compute_pipeline: &wgpu::ComputePipeline,
        compute_bind_group: &BindGroup,
    ) {
        let mut compute_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Encoder"),
                });

        // ###################################### update accumulation ########################################
        let width = self.camera.viewport_width;
        let height = self.camera.viewport_height;

        if self.accumulate {
            let params = Params {
                sky_color: self.scene.sky_color,
                width,
                accumulation_index: self.accumulation_index,
                accumulate: self.accumulate as u32,
                sphere_count: self.scene.spheres.len() as u32,
                object_count: self.scene.objects.len() as u32,
                compute_per_frame: self.compute_per_frame,
                _padding: [0; 12],
            };

            self.buffers.update_accumulation(self.queue, &[params]);

            self.accumulation_index += 1;
        }
        // ###################################### compute step ########################################

        {
            let mut compute_pass =
                compute_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Compute Pass"),
                    timestamp_writes: None,
                });
            compute_pass.set_pipeline(compute_pipeline);
            compute_pass.set_bind_group(0, compute_bind_group, &[]);
            compute_pass.dispatch_workgroups((width + 7) / 8, (height + 7) / 8, 1);
        }

        self.queue.submit(Some(compute_encoder.finish()));

        // ###################################### copying buffers to texture ########################################
    }

    pub fn update_texture(&mut self, encoder: &mut CommandEncoder, texture: &Texture) {
        // ###################################### update accumulation ########################################
        let width = self.camera.viewport_width;
        let height = self.camera.viewport_height;

        let bytes_per_row = self.calculate_bytes_per_row(width);

        // Copy the GPU output buffer to the texture to be displayed
        encoder.copy_buffer_to_texture(
            wgpu::ImageCopyBuffer {
                buffer: &self.buffers.output_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row), // 4 bytes per pixel
                    rows_per_image: Some(height),
                },
            },
            wgpu::ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn calculate_bytes_per_row(&self, width: u32) -> u32 {
        // the gpu buffer has to be 256 * n bytes per row

        let bytes_per_pixel = 4; // RGBA8Unorm = 4 bytes per pixel

        let value = width * bytes_per_pixel;
        let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

        // bytes per row
        (value + alignment - 1) & !(alignment - 1)
    }
}

pub fn get_triangle_data(scene: &RenderScene) -> (Vec<ObjectInfo>, Vec<SceneTriangle>) {
    let object_info_vec: Vec<ObjectInfo> = scene
        .objects
        .iter()
        .map(|object| object.object_info)
        .collect();

    let triangles: Vec<_> = scene
        .objects
        .iter()
        .flat_map(|object| object.object_triangles.clone())
        .collect();
    (object_info_vec, triangles)
}

pub fn make_texture_array(device: &Device, queue: &Queue) -> Texture {
    let texture_count = 6; // Example number of textures
    let texture_size = wgpu::Extent3d {
        width: 100,
        height: 100,
        depth_or_array_layers: texture_count,
    };

    let texture_array = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Texture Array"),
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    for i in 0..texture_count {
        let img = image::open("./textures/red.png").unwrap();
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture_array,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: i },
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0), // 4x u8 per pixel
                rows_per_image: Some(dimensions.1),
            },
            wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 1,
            },
        );
    }

    texture_array
}
