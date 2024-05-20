use crate::buffers::{Params, SceneMaterial, SceneSphere};

use super::camera::{Camera, Ray};
use super::Scene::RenderScene;

use super::buffers;

use egui::Context;

use glam::{vec3a, Vec3A};

use rayon::{prelude::*, ThreadPool, ThreadPoolBuilder};

use wgpu::{BindGroup, BindGroupLayout, Queue, Texture};

#[derive(Debug, Clone, Copy, PartialEq)]
struct HitPayload {
    hit_distance: f32,
    world_position: Vec3A,
    world_normal: Vec3A,

    object_index: usize,
}

fn generate_new_camera_rays(camera: &Camera) -> Vec<buffers::Ray> {
    let mut ray_vec: Vec<buffers::Ray> = vec![];

    let rays = camera.ray_directions.clone();

    for ray in rays {
        let direction = ray.direction;
        let new_ray = buffers::Ray {
            direction: [direction.x, direction.y, direction.z],
            _padding: [0; 4],
        };

        ray_vec.push(new_ray)
    }

    ray_vec
}

pub struct Renderer {
    pub camera: Camera,
    pub scene: RenderScene,

    pub accumulate: bool,
    pub light_mode: u32,
    accumulation_index: u32,
    buffers: buffers::DataBuffers,

    // ###############
    accumulated_image: Vec<Vec3A>,
    pub thread_pool: ThreadPool,
}

impl Renderer {
    pub fn new(
        camera: Camera,
        scene: RenderScene,
        device: &wgpu::Device,
        size: &winit::dpi::PhysicalSize<u32>,

        material_array: &[SceneMaterial],
        sphere_array: &[SceneSphere],
        params: &[Params],
    ) -> (Renderer, BindGroupLayout, BindGroup) {
        let available_threads = rayon::current_num_threads();
        let used_threads = available_threads / 2;

        let thread_pool = ThreadPoolBuilder::new()
            .num_threads(used_threads)
            .build()
            .expect("couldn't construct threadpool");

        let camera_rays = generate_new_camera_rays(&camera);

        let (buffers, bind_group_layout, compute_bind_group) = buffers::DataBuffers::new(
            device,
            size,
            &camera_rays,
            material_array,
            sphere_array,
            params,
        );

        let mut renderer = Renderer {
            camera,
            scene,

            accumulate: true,
            light_mode: 0,
            accumulated_image: vec![],
            accumulation_index: 1,
            thread_pool,
            buffers,
        };

        renderer.reset_accumulation();

        (renderer, bind_group_layout, compute_bind_group)
    }

    pub fn _generate_pixels(&mut self) -> Vec<u8> {
        let ray_directions = &self.camera.ray_directions;

        let mut pixel_rgba: Vec<u8> = Vec::with_capacity(ray_directions.len() * 4);

        let n_bounces = 4;

        let new_colors: Vec<Vec3A> = (0..ray_directions.len())
            .into_par_iter()
            .map(|index| self._per_pixel(index, n_bounces))
            .collect();

        for (index, color) in new_colors.iter().enumerate() {
            self.accumulated_image[index] += *color;
        }

        self.thread_pool.install(|| {
            pixel_rgba = (0..ray_directions.len())
                .into_par_iter()
                .flat_map_iter(|index: usize| {
                    let normalized_color =
                        self.accumulated_image[index] / self.accumulation_index as f32;

                    self._to_rgba(normalized_color)
                })
                .collect();
        });

        if self.accumulate {
            self.accumulation_index += 1;
        } else {
            self.reset_accumulation();
        }

        pixel_rgba
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

    fn _trace_ray(&self, ray: &Ray) -> HitPayload {
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
    }

    fn _closest_hit(&self, ray: &Ray, hit_distance: f32, object_index: usize) -> HitPayload {
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
    }

    fn _miss(&self, _ray: &Ray) -> HitPayload {
        HitPayload {
            hit_distance: -1.0,
            world_position: Vec3A::splat(0.),
            world_normal: Vec3A::splat(0.),
            object_index: 0,
        }
    }

    fn _per_pixel(&self, index: usize, bounces: u8) -> Vec3A {
        let mut ray = self.camera.ray_directions[index];
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

        light
    }

    fn _reflect_ray(&self, ray: Vec3A, normal: Vec3A) -> Vec3A {
        ray - (2.0 * ray.dot(normal) * normal)
    }

    fn _to_rgba(&self, mut vector: Vec3A) -> [u8; 4] {
        vector *= 255.0;
        [vector.x as u8, vector.y as u8, vector.z as u8, 255]
    }

    pub fn on_resize(&mut self, width: u32, height: u32) {
        self.camera.on_resize(width, height);

        self.reset_accumulation()
    }

    pub fn on_update(&mut self, mouse_delta: egui::Vec2, timestep: &f32, egui_context: &Context) {
        let moved = self.camera.on_update(mouse_delta, timestep, egui_context);
        if moved {
            self.reset_accumulation()
        };
    }

    pub fn reset_accumulation(&mut self) {
        let total_size = (self.camera.viewport_height * self.camera.viewport_width) as usize;
        self.accumulated_image = vec![Vec3A::splat(0.0); total_size];

        self.accumulation_index = 1;
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

    pub fn update_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &Queue,
        compute_pipeline: &wgpu::ComputePipeline,
        compute_bind_group: &BindGroup,
        texture: &Texture,
    ) {
        // ###################################### update accumulation ########################################
        let width = self.camera.viewport_width;
        let height = self.camera.viewport_height;

        if self.accumulate {
            self.accumulation_index += 1;

            let params = Params {
                width,
                accumulation_index: self.accumulation_index,
                _padding: [0; 8],
            };

            queue.write_buffer(
                &self.buffers.params_buffer,
                0,
                bytemuck::cast_slice(&[params]),
            );
        }
        // ###################################### compute step ########################################

        let mut compute_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Compute Encoder"),
        });

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
        let mut copy_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Copy Encoder"),
        });

        let bytes_per_row = self.calculate_bytes_per_row(width);

        // Copy the output buffer to the texture
        copy_encoder.copy_buffer_to_texture(
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

        queue.submit(Some(copy_encoder.finish()));
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
