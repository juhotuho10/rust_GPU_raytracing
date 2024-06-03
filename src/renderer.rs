use crate::buffers::{
    ObjectInfo, Params, RayCamera, SceneMaterial, SceneSphere, SceneTriangle, SubObjectInfo,
};

use crate::triangle_object::SceneObject;

use crate::image_texture::ImageTexture;

use super::camera::Camera;

use super::buffers;

use egui::Context;

use wgpu::{BindGroup, BindGroupLayout, CommandEncoder, Device, Queue, Texture};

#[derive(Debug, Clone)]
pub struct RenderScene {
    pub spheres: Vec<SceneSphere>,
    pub texture_size: [u32; 2],
    pub image_textures: Vec<ImageTexture>,
    pub materials: Vec<SceneMaterial>,
    pub objects: Vec<SceneObject>,
    pub environment_map: ImageTexture,
    pub env_map_size: [u32; 2],
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

        let (object_info_vec, sub_object_info_vec, triangles) = get_triangle_data(&scene);

        let ray_camera: RayCamera = RayCamera {
            origin: camera.position.into(),
            _padding: [0; 4],
        };

        let (buffers, bind_group_layout, compute_bind_group) = buffers::DataBuffers::new(
            device,
            &size,
            ray_camera,
            &camera_rays,
            &scene.materials,
            &scene.spheres,
            &triangles,
            &object_info_vec,
            &sub_object_info_vec,
            &[params],
        );

        buffers.update_texture_buffer(
            &scene.image_textures,
            queue,
            scene.texture_size[0],
            scene.texture_size[1],
        );

        buffers.update_environment_map_buffer(
            &scene.environment_map,
            queue,
            scene.env_map_size[0],
            scene.env_map_size[1],
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
            screen_width: self.camera.viewport_width,
            accumulation_index: self.accumulation_index,
            accumulate: self.accumulate as u32,
            sphere_count: self.scene.spheres.len() as u32,
            object_count: self.scene.objects.len() as u32,
            compute_per_frame: self.compute_per_frame,
            texture_width: self.scene.texture_size[0],
            texture_height: self.scene.texture_size[1],
            textue_count: self.scene.image_textures.len() as u32,
            env_map_width: self.scene.env_map_size[0],
            env_map_height: self.scene.env_map_size[1],
            _padding: [0; 4],
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

        for texture in &mut self.scene.image_textures {
            texture.update_color();
        }

        self.scene.environment_map.update_color();

        for object in &mut self.scene.objects {
            object.update_sub_objects();
        }

        self.buffers.update_texture_buffer(
            &self.scene.image_textures,
            self.queue,
            self.scene.texture_size[0],
            self.scene.texture_size[1],
        );

        self.buffers.update_environment_map_buffer(
            &self.scene.environment_map,
            self.queue,
            self.scene.env_map_size[0],
            self.scene.env_map_size[1],
        );

        let (new_object_info, old_sub_object_info, new_triangles) = get_triangle_data(&self.scene);

        self.buffers.update_triangles(self.queue, &new_triangles);

        self.buffers
            .update_object_info(self.queue, &new_object_info);

        self.buffers
            .update_sub_object_info(self.queue, &old_sub_object_info);

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
                screen_width: width,
                accumulation_index: self.accumulation_index,
                accumulate: self.accumulate as u32,
                sphere_count: self.scene.spheres.len() as u32,
                object_count: self.scene.objects.len() as u32,
                compute_per_frame: self.compute_per_frame,
                texture_width: self.scene.texture_size[0],
                texture_height: self.scene.texture_size[1],
                textue_count: self.scene.image_textures.len() as u32,
                env_map_width: self.scene.env_map_size[0],
                env_map_height: self.scene.env_map_size[1],
                _padding: [0; 4],
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

pub fn get_triangle_data(
    scene: &RenderScene,
) -> (Vec<ObjectInfo>, Vec<SubObjectInfo>, Vec<SceneTriangle>) {
    let object_info_vec: Vec<ObjectInfo> = scene
        .objects
        .iter()
        .map(|object| object.object_info)
        .collect();

    // generate new object info
    let sub_object_info_vec: Vec<SubObjectInfo> = scene
        .objects
        .iter()
        .flat_map(|object| object.sub_object_info.clone())
        .collect();

    let triangles: Vec<_> = scene
        .objects
        .iter()
        .flat_map(|object| object.object_triangles.clone())
        .collect();
    (object_info_vec, sub_object_info_vec, triangles)
}
