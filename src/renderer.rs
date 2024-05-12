use crate::Scene::Sphere;

use super::camera::{Camera, Ray};
use super::Scene::RenderScene;

use egui::Context;

use glam::{vec3a, Vec3A};

use rand::distributions::{Distribution, Uniform};
use rand::Rng;
use rayon::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
struct HitPayload {
    hit_distance: f32,
    world_position: Vec3A,
    world_normal: Vec3A,

    object_index: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Renderer {
    pub camera: Camera,
    pub scene: RenderScene,
    pub light_accumulation: bool,
}

impl Renderer {
    pub fn generate_pixels(
        &self,
        rng: &mut rand::rngs::ThreadRng,
        thread_pool: &rayon::ThreadPool,
    ) -> Vec<u8> {
        let ray_directions = &self.camera.ray_directions;

        let mut pixel_colors: Vec<u8> = Vec::with_capacity(ray_directions.len() * 4);

        let n_bounces = 2;

        thread_pool.install(|| {
            pixel_colors = (0..ray_directions.len())
                .into_par_iter()
                .flat_map_iter(|index| {
                    let color_rgba = self.per_pixel(index, n_bounces);

                    color_rgba.into_iter()
                })
                .collect();
        });
        pixel_colors
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

    fn miss(&self, ray: &Ray) -> HitPayload {
        HitPayload {
            hit_distance: -1.0,
            world_position: Vec3A::splat(0.),
            world_normal: Vec3A::splat(0.),
            object_index: 0,
        }
    }

    fn per_pixel(&self, index: usize, bounces: u8) -> [u8; 4] {
        let mut ray = self.camera.ray_directions[index];
        let mut multiplier = 1.0;
        let mut final_color = Vec3A::splat(0.);
        let mut rng = rand::thread_rng();

        let mut random_scatter: Vec3A = rng.gen();

        for i in 0..bounces {
            let hit_payload = self.trace_ray(&ray);

            if hit_payload.hit_distance < 0. {
                // missed sphere
                let skycolor = vec3a(0.6, 0.7, 0.9);
                final_color += skycolor * multiplier;
                break;
            }

            let light_direction = vec3a(1., 1., -1.).normalize();
            //cosine of the angle between hitpoin and the light direction
            //min light intenstiy is 0.05
            let light_intensity = hit_payload.world_normal.dot(-light_direction).max(0.05);

            let hit_idex = hit_payload.object_index;
            let closest_sphere = self.scene.spheres[hit_idex];
            let mut sphere_color = closest_sphere.material.albedo;
            sphere_color *= light_intensity;

            final_color += sphere_color * multiplier;

            multiplier *= 0.7;

            // move new ray origin to the position of the hit
            // move a little bit towards he normal so that the ray isnt cast from within the wall
            ray.origin = hit_payload.world_position + hit_payload.world_normal * 0.0001;

            // scale random values between -0.5 and 0.5
            random_scatter -= 0.5;

            let reflected_ray: Vec3A = self.reflect_ray(
                ray.direction,
                hit_payload.world_normal + closest_sphere.material.roughness * random_scatter,
            );
            ray.direction = reflected_ray;
        }

        self.to_rgba(final_color)
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
    }

    pub fn on_update(
        &mut self,
        mouse_delta: egui::Vec2,
        timestep: &f32,
        egui_context: &Context,
    ) -> bool {
        self.camera.on_update(mouse_delta, timestep, egui_context)
    }
}