use crate::buffers::{Params, RayCamera, SceneMaterial, SceneSphere, SceneTriangle};

use super::camera::Camera;

use super::buffers;

use egui::Context;

use wgpu::{BindGroup, BindGroupLayout, CommandEncoder, Device, Queue, Texture};

#[derive(Debug, Clone)]
pub struct RenderScene {
    pub spheres: Vec<SceneSphere>,
    pub triangles: Vec<SceneTriangle>,
    pub materials: Vec<SceneMaterial>,
    pub sky_color: [f32; 3],
}

pub struct Renderer {
    pub camera: Camera,
    pub scene: RenderScene,
    pub accumulate: bool,
    pub light_mode: u32,
    accumulation_index: u32,
    buffers: buffers::DataBuffers,
}

impl Renderer {
    pub fn new(
        camera: Camera,
        scene: RenderScene,
        device: &Device,
        size: &winit::dpi::PhysicalSize<u32>,
        params: Params,
    ) -> (Renderer, BindGroupLayout, BindGroup) {
        let camera_rays = camera.recalculate_ray_directions();

        let accumulate = params.accumulate == 1;

        let (buffers, bind_group_layout, compute_bind_group) = buffers::DataBuffers::new(
            device,
            size,
            &camera_rays,
            &scene.materials,
            &scene.spheres,
            &scene.triangles,
            &[params],
        );

        let renderer = Renderer {
            camera,
            scene,

            accumulate,
            light_mode: 0,
            accumulation_index: 1,
            buffers,
        };

        (renderer, bind_group_layout, compute_bind_group)
    }

    fn _reflect_ray(&self, ray: glam::Vec3A, normal: glam::Vec3A) -> glam::Vec3A {
        ray - (2.0 * ray.dot(normal) * normal)
    }

    pub fn on_resize(
        &mut self,
        size: &winit::dpi::PhysicalSize<u32>,
        device: &Device,
        queue: &Queue,
    ) {
        self.camera.on_resize(size.width, size.height);

        self.reset_accumulation(device, queue)
    }

    pub fn on_update(
        &mut self,
        device: &Device,
        queue: &Queue,
        mouse_delta: egui::Vec2,
        timestep: &f32,
        egui_context: &Context,
    ) {
        let moved = self.camera.on_update(mouse_delta, timestep, egui_context);

        if moved {
            self.reset_accumulation(device, queue);
            let new_rays = self.camera.recalculate_ray_directions();

            let new_camera = RayCamera {
                origin: self.camera.position.into(),
                _padding: [0; 4],
            };

            queue.write_buffer(
                &self.buffers.camera_buffer,
                0,
                bytemuck::cast_slice(&[new_camera]),
            );

            self.buffers.update_ray_directions(queue, &new_rays);
        };
    }

    pub fn reset_accumulation(&mut self, device: &Device, queue: &Queue) {
        self.accumulation_index = 1;

        let params = Params {
            sky_color: self.scene.sky_color,
            width: self.camera.viewport_width,
            accumulation_index: self.accumulation_index,
            accumulate: self.accumulate as u32,

            _padding: [0; 8],
        };

        self.buffers.reset_accumulation(device, queue, &[params]);
    }

    pub fn update_scene(&mut self, device: &Device, queue: &Queue) {
        self.reset_accumulation(device, queue);

        let new_spheres = &self.scene.spheres;
        self.buffers.update_spheres(queue, new_spheres);

        let new_triangles = &self.scene.triangles;
        self.buffers.update_triangles(queue, new_triangles);

        let new_materials = &self.scene.materials;
        self.buffers.update_materials(queue, new_materials);
    }

    pub fn update_frame(
        &mut self,
        encoder: &mut CommandEncoder,
        queue: &Queue,
        compute_pipeline: &wgpu::ComputePipeline,
        compute_bind_group: &BindGroup,
        texture: &Texture,
    ) {
        // ###################################### update accumulation ########################################
        let width = self.camera.viewport_width;
        let height = self.camera.viewport_height;

        if self.accumulate {
            let params = Params {
                sky_color: self.scene.sky_color,
                width: self.camera.viewport_width,
                accumulation_index: self.accumulation_index,
                accumulate: self.accumulate as u32,

                _padding: [0; 8],
            };

            self.buffers.update_accumulation(queue, &[params]);

            self.accumulation_index += 1;
        }
        // ###################################### compute step ########################################

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(compute_pipeline);
            compute_pass.set_bind_group(0, compute_bind_group, &[]);
            compute_pass.dispatch_workgroups((width + 7) / 8, (height + 7) / 8, 1);
        }

        // ###################################### copying buffers to texture ########################################

        let bytes_per_row = self.calculate_bytes_per_row(width);

        // Copy the output buffer to the texture
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
