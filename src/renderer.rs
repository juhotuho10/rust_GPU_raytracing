use crate::buffers::{Params, RayCamera, SceneMaterial, SceneSphere};

use super::camera::Camera;

use super::buffers;

use egui::Context;

use glam::{vec3a, Vec3A};

use rayon::{ThreadPool, ThreadPoolBuilder};

use wgpu::{BindGroup, BindGroupLayout, CommandEncoder, Device, Queue, Texture};

#[derive(Debug, Clone)]
pub struct RenderScene {
    pub spheres: Vec<SceneSphere>,
    pub materials: Vec<SceneMaterial>,
    pub sky_color: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct HitPayload {
    hit_distance: f32,
    world_position: Vec3A,
    world_normal: Vec3A,

    object_index: usize,
}

pub struct Renderer {
    pub camera: Camera,
    pub scene: RenderScene,
    pub accumulate: bool,
    pub light_mode: u32,
    accumulation_index: u32,
    buffers: buffers::DataBuffers,

    // ###############
    pub thread_pool: ThreadPool,
}

impl Renderer {
    pub fn new(
        camera: Camera,
        scene: RenderScene,
        device: &Device,
        size: &winit::dpi::PhysicalSize<u32>,
        params: &[Params],
    ) -> (Renderer, BindGroupLayout, BindGroup) {
        let available_threads = rayon::current_num_threads();
        let used_threads = available_threads / 2;

        let thread_pool = ThreadPoolBuilder::new()
            .num_threads(used_threads)
            .build()
            .expect("couldn't construct threadpool");

        let camera_rays = camera.recalculate_ray_directions();

        let (buffers, bind_group_layout, compute_bind_group) = buffers::DataBuffers::new(
            device,
            size,
            &camera_rays,
            &scene.materials,
            &scene.spheres,
            params,
        );

        let renderer = Renderer {
            camera,
            scene,

            accumulate: true,
            light_mode: 0,

            accumulation_index: 1,
            thread_pool,
            buffers,
        };

        (renderer, bind_group_layout, compute_bind_group)
    }

    // single threadded version of the rendering for testing
    /*pub fn generate_pixels(
        &self,
        rng: &mut rand::rngs::ThreadRng,
        thread_pool: &rayon::ThreadPool,
    ) -> Vec<u8> {
        let mut pixel_colors: Vec<u8> = Vec::with_capacity(&self.camera.ray_directions.len() * 4);
        let ray_directions = &self.camera.ray_directions;

        for index in 0..ray_directions.len() {
            let color_rgba = self.per_pixel(index, 2);
            pixel_colors.extend_from_slice(&color_rgba);
        }
        pixel_colors
    }*/

    /*fn _trace_ray(&self, ray: &Ray) -> HitPayload {
        // (bx^2 + by^2)t^2 + 2*(axbx + ayby)t + (ax^2 + by^2 - r^2) = 0
        // where
        // a = ray origin
        // b = ray direction
        // r = sphere radius
        // t = hit distance

        //dbg!(ray.direction);

        let mut hit_distance = f32::MAX;
        let mut closest_sphere_index: Option<usize> = None;

        let a: f32 = ray.direction.dot(ray.direction);

        for sphere_index in 0..self.scene.spheres.len() {
            let sphere = self.scene.spheres[sphere_index];

            let origin = ray.origin - sphere.position;

            let b: f32 = 2.0 * ray.direction.dot(origin);
            let c: f32 = origin.dot(origin) - (sphere.radius * sphere.radius);

            // discriminant:
            // b^2 - 4*a*c
            let discriminant = b * b - 4. * a * c;

            if discriminant < 0. {
                // we missed the sphere
                continue;
            }
            // (-b +- discriminant) / 2a
            //let t0 = (-b + discriminant.sqrt()) / (2. * a);

            let current_t = (-b - discriminant.sqrt()) / (2. * a);
            if current_t > 0.0 && current_t < hit_distance {
                hit_distance = current_t;
                closest_sphere_index = Some(sphere_index);
            }
        }

        match closest_sphere_index {
            None => self._miss(ray),
            Some(sphere_index) => self._closest_hit(ray, hit_distance, sphere_index),
        }
    }*/

    /*fn _closest_hit(&self, ray: &Ray, hit_distance: f32, object_index: usize) -> HitPayload {
        let closest_sphere = &self.scene.spheres[object_index];

        //let origin = ray.origin - closest_sphere.position;
        let hit_point = ray.origin + ray.direction * hit_distance;
        let sphere_normal = (hit_point - closest_sphere.position).normalize();

        HitPayload {
            hit_distance,
            world_position: hit_point,
            world_normal: sphere_normal,
            object_index,
        }
    }*/

    fn _per_pixel(&self, index: usize, bounces: u8) -> Vec3A {
        /*let mut ray = self.camera.recalculate_ray_directions();
        let mut light_contribution = Vec3A::splat(1.0);
        let mut light = Vec3A::splat(0.0);

        let mut seed = (index as u32) * (self.accumulation_index * 326624);

        for _ in 0..bounces {
            let hit_payload = &self._trace_ray(&ray);

            if hit_payload.hit_distance < 0. {
                // missed sphere, we het ambient color

                light += self.scene.sky_color * light_contribution;
                break;
            }

            let hit_idex = hit_payload.object_index;
            let closest_sphere = &self.scene.spheres[hit_idex];
            let material_index = closest_sphere.material_index;
            let current_material = &self.scene.materials[material_index];

            light += current_material._get_emission() * light_contribution;

            light_contribution *= current_material.albedo * current_material.metallic;

            // move new ray origin to the position of the hit
            // move a little bit towards he normal so that the ray isnt cast from within the wall
            ray.origin = hit_payload.world_position + hit_payload.world_normal * 0.0001;

            match self.light_mode {
                0 => {
                    ray.direction = self
                        ._reflect_ray(
                            ray.direction,
                            hit_payload.world_normal
                                + current_material.roughness * self._random_scaler(&mut seed),
                        )
                        .normalize();
                }
                1 => {
                    ray.direction = (self._reflect_ray(ray.direction, hit_payload.world_normal)
                        + current_material.roughness * self._random_scaler(&mut seed))
                    .normalize();
                }
                2 => {
                    ray.direction = (self._reflect_ray(ray.direction, hit_payload.world_normal)
                        + ((hit_payload.world_normal + self._random_scaler(&mut seed))
                            * current_material.roughness))
                        .normalize()
                }
                3 => {
                    ray.direction =
                        (hit_payload.world_normal + self._random_scaler(&mut seed)).normalize()
                }
                _ => {
                    unimplemented!("light mode doesnt exist")
                }
            }
        }
        */
        Vec3A::splat(0.0)
    }

    fn _reflect_ray(&self, ray: Vec3A, normal: Vec3A) -> Vec3A {
        ray - (2.0 * ray.dot(normal) * normal)
    }

    fn _to_rgba(&self, mut vector: Vec3A) -> [u8; 4] {
        vector *= 255.0;
        [vector.x as u8, vector.y as u8, vector.z as u8, 255]
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

    fn _pcg_hash(&self, seed: &mut u32) -> f32 {
        let state = seed.wrapping_mul(747796405).wrapping_add(2891336453);

        let word =
            ((state.wrapping_shr((state.wrapping_shr(28)) + 4)) ^ state).wrapping_mul(277803737);

        *seed = word.wrapping_shr(22) ^ word;

        *seed as f32
    }

    fn _random_scaler(&self, seed: &mut u32) -> Vec3A {
        self._positive_random_scaler(seed) * 2.0 - 1.0
    }

    fn _positive_random_scaler(&self, seed: &mut u32) -> Vec3A {
        let scaler = vec3a(
            self._pcg_hash(seed),
            self._pcg_hash(seed),
            self._pcg_hash(seed),
        );

        scaler / (u32::MAX as f32)
    }

    pub fn update_scene(&mut self, device: &Device, queue: &Queue) {
        self.reset_accumulation(device, queue);

        let new_spheres = &self.scene.spheres;
        self.buffers.update_spheres(queue, new_spheres);

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
