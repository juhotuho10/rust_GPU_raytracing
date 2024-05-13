use std::fs::DirBuilder;

use crate::Scene::{Material, Sphere};

use super::camera::{Camera, Ray};
use super::Scene::RenderScene;

use egui::Context;

use glam::{vec3a, Vec3A};

use rayon::prelude::*;
use wgpu::hal::auxil::db;

#[derive(Debug, Clone, Copy, PartialEq)]
struct HitPayload {
    hit_distance: f32,
    world_position: Vec3A,
    world_normal: Vec3A,

    object_index: usize,
}

#[derive(Debug)]
pub struct Renderer {
    pub camera: Camera,
    pub scene: RenderScene,
    pub accumulate: bool,
    pub light_mode: u32,
    accumulated_image: Vec<Vec3A>,
    accumulation_index: f32,
    frame_index: u32,
}

impl Renderer {
    pub fn new(camera: Camera, scene: RenderScene) -> Renderer {
        let mut renderer = Renderer {
            camera,
            scene,
            accumulate: true,
            light_mode: 0,
            accumulated_image: vec![],
            accumulation_index: 1.0,
            frame_index: 0,
        };

        renderer.reset_accumulation();

        renderer
    }

    pub fn generate_pixels(&mut self, thread_pool: &rayon::ThreadPool) -> Vec<u8> {
        let ray_directions = &self.camera.ray_directions;

        let mut pixel_rgba: Vec<u8> = Vec::with_capacity(ray_directions.len() * 4);

        let n_bounces = 4;

        let new_colors: Vec<Vec3A> = (0..ray_directions.len())
            .into_par_iter()
            .map(|index| self.per_pixel(index, n_bounces))
            .collect();

        for (index, color) in new_colors.iter().enumerate() {
            self.accumulated_image[index] += *color;
        }

        thread_pool.install(|| {
            pixel_rgba = (0..ray_directions.len())
                .into_par_iter()
                .flat_map_iter(|index: usize| {
                    let normalized_color = self.accumulated_image[index] / self.accumulation_index;

                    self.to_rgba(normalized_color)
                })
                .collect();
        });

        if self.accumulate {
            self.accumulation_index += 1.0;
        } else {
            self.reset_accumulation();
        }

        self.frame_index += 1;

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

    fn trace_ray(&self, ray: &Ray) -> HitPayload {
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
            None => self.miss(ray),
            Some(sphere_index) => self.closest_hit(ray, hit_distance, sphere_index),
        }
    }

    fn closest_hit(&self, ray: &Ray, hit_distance: f32, object_index: usize) -> HitPayload {
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

    fn miss(&self, _ray: &Ray) -> HitPayload {
        HitPayload {
            hit_distance: -1.0,
            world_position: Vec3A::splat(0.),
            world_normal: Vec3A::splat(0.),
            object_index: 0,
        }
    }

    fn per_pixel(&self, index: usize, bounces: u8) -> Vec3A {
        let mut ray = self.camera.ray_directions[index];
        let mut light_contribution = Vec3A::splat(1.0);
        let mut light = Vec3A::splat(0.0);

        let mut seed = index as u32 * self.frame_index;

        for i in 0..bounces {
            let hit_payload = &self.trace_ray(&ray);

            if hit_payload.hit_distance < 0. {
                // missed sphere
                //let sky_light = vec3a(0.6, 0.7, 0.9);
                //light += sky_light * light_contribution;
                break;
            }

            let hit_idex = hit_payload.object_index;
            let closest_sphere = &self.scene.spheres[hit_idex];
            let material_index = closest_sphere.material_index;
            let current_material = &self.scene.materials[material_index];

            light_contribution *= current_material.albedo;
            light += current_material.get_emission() * light_contribution;

            // move new ray origin to the position of the hit
            // move a little bit towards he normal so that the ray isnt cast from within the wall
            ray.origin = hit_payload.world_position + hit_payload.world_normal * 0.0001;

            match self.light_mode {
                0 => {
                    ray.direction = self
                        .reflect_ray(
                            ray.direction,
                            hit_payload.world_normal
                                + current_material.roughness * self.random_scaler(&mut seed),
                        )
                        .normalize();
                }
                1 => {
                    ray.direction = (self.reflect_ray(ray.direction, hit_payload.world_normal)
                        + current_material.roughness * self.random_scaler(&mut seed))
                    .normalize();
                }
                2 => {
                    ray.direction = (self.reflect_ray(ray.direction, hit_payload.world_normal)
                        + ((hit_payload.world_normal + self.random_scaler(&mut seed))
                            * current_material.roughness))
                        .normalize()
                }
                3 => {
                    ray.direction =
                        (hit_payload.world_normal + self.random_scaler(&mut seed)).normalize()
                }
                _ => {
                    unimplemented!("light mode doesnt exist")
                }
            }
        }

        light
    }

    fn reflect_ray(&self, ray: Vec3A, normal: Vec3A) -> Vec3A {
        ray - (2.0 * ray.dot(normal) * normal)
    }

    fn to_rgba(&self, mut vector: Vec3A) -> [u8; 4] {
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

        self.accumulation_index = 1.0;
    }

    fn pcg_hash(&self, seed: &mut u32) -> f32 {
        let state = *seed * 747796405 + 2891336453;
        let word = ((state >> ((state >> 28) + 4)) ^ state) * 277803737;

        *seed = (word >> 22) ^ word;

        *seed as f32
    }

    fn random_scaler(&self, seed: &mut u32) -> Vec3A {
        self.positive_random_scaler(seed) * 2.0 - 1.0
    }

    fn positive_random_scaler(&self, seed: &mut u32) -> Vec3A {
        let scaler = vec3a(
            self.pcg_hash(seed),
            self.pcg_hash(seed),
            self.pcg_hash(seed),
        );

        scaler / (u32::MAX as f32)
    }
}
