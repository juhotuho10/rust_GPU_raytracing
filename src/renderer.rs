use crate::buffers::{ObjectInfo, Params, RayCamera, SceneMaterial, SceneSphere, SceneTriangle};

use crate::stl_to_triangles::SceneObject;

use super::camera::Camera;

use super::buffers;

use egui::Context;

use wgpu::{BindGroup, BindGroupLayout, CommandEncoder, Device, Queue, Texture};

#[derive(Debug, Clone)]
pub struct RenderScene {
    pub spheres: Vec<SceneSphere>,
    pub materials: Vec<SceneMaterial>,
    pub objects: Vec<SceneObject>,
    pub sky_color: [f32; 3],
}

pub struct Renderer {
    pub camera: Camera,
    pub scene: RenderScene,
    pub accumulate: bool,
    pub material_index: usize,
    pub object_index: usize,
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

        let (object_info_vec, triangles) = get_triangle_data(&scene);

        let (buffers, bind_group_layout, compute_bind_group) = buffers::DataBuffers::new(
            device,
            size,
            &camera_rays,
            &scene.materials,
            &scene.spheres,
            &triangles,
            &object_info_vec,
            &[params],
        );

        let renderer = Renderer {
            camera,
            scene,
            accumulate,
            material_index: 0,
            object_index: 0,
            accumulation_index: 1,
            buffers,
        };

        (renderer, bind_group_layout, compute_bind_group)
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

        egui_context: &Context,
    ) {
        let moved = self.camera.on_update(mouse_delta, egui_context);

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
            sphere_count: self.scene.spheres.len() as u32,
            triangle_count: self.get_triangle_count(),
        };

        self.buffers.reset_accumulation(device, queue, &[params]);
    }

    pub fn update_scene(&mut self, device: &Device, queue: &Queue) {
        self.reset_accumulation(device, queue);

        let new_spheres = &self.scene.spheres;
        self.buffers.update_spheres(queue, new_spheres);

        for object in &mut self.scene.objects {
            object.update_triangles();
        }
        let (new_object_info, new_triangles) = get_triangle_data(&self.scene);

        self.buffers.update_triangles(queue, &new_triangles);

        self.buffers.update_object_info(queue, &new_object_info);

        let new_materials = &self.scene.materials;
        self.buffers.update_materials(queue, new_materials);
    }

    pub fn compute_frame(
        &mut self,
        device: &Device,
        queue: &Queue,
        compute_pipeline: &wgpu::ComputePipeline,
        compute_bind_group: &BindGroup,
    ) {
        let mut compute_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
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
                triangle_count: self.get_triangle_count(),
            };

            self.buffers.update_accumulation(queue, &[params]);

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

        queue.submit(Some(compute_encoder.finish()));

        // ###################################### copying buffers to texture ########################################
    }

    pub fn update_texture(&mut self, encoder: &mut CommandEncoder, texture: &Texture) {
        // ###################################### update accumulation ########################################
        let width = self.camera.viewport_width;
        let height = self.camera.viewport_height;

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

    pub fn get_triangle_count(&self) -> u32 {
        self.scene
            .objects
            .iter()
            .map(|obj: &SceneObject| obj.object_triangles.len())
            .sum::<usize>() as u32
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
