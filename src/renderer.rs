use super::camera::{Camera, Ray};
use super::Scene::{RenderScene, Sphere};

use egui::Context;

use glam::{quat, vec3a, Quat, Vec3, Vec3A, Vec4};
use rayon::prelude::*;
#[derive(Debug, Clone, Copy, PartialEq)]
struct HitPayload {}

#[derive(Debug, Clone, PartialEq)]
pub struct Renderer {
    pub camera: Camera,
    pub scene: RenderScene,
}

impl Renderer {
    pub fn new(camera: Camera, scene: RenderScene) -> Renderer {
        Renderer { camera, scene }
    }

    pub fn generate_pixels(
        &self,
        rng: &mut rand::rngs::ThreadRng,
        thread_pool: &rayon::ThreadPool,
    ) -> Vec<u8> {
        let camera_pos = self.camera.position;
        let ray_directions = &self.camera.ray_directions;

        let mut pixel_colors: Vec<u8> = Vec::with_capacity(ray_directions.len() * 4);

        thread_pool.install(|| {
            pixel_colors = (0..ray_directions.len())
                .into_par_iter()
                .flat_map_iter(|index| {
                    let color = self.trace_ray(&self.scene, &ray_directions[index]);

                    let color_rgba = self.to_rgba(color);

                    color_rgba.into_iter()
                })
                .collect();
        });
        pixel_colors
    }

    pub fn trace_ray(&self, scene: &RenderScene, ray: &Ray) -> Vec3A {
        // (bx^2 + by^2)t^2 + 2*(axbx + ayby)t + (ax^2 + by^2 - r^2) = 0
        // where
        // a = ray origin
        // b = ray direction
        // r = sphere radius
        // t = hit distance

        //dbg!(ray.direction);

        let clear_color = vec3a(0., 0., 0.);

        let mut hit_distance = f32::MAX;
        let mut closest_sphere: Option<&Sphere> = None;

        if scene.spheres.is_empty() {
            return clear_color;
        }

        let a: f32 = ray.direction.dot(ray.direction);

        for sphere in &scene.spheres {
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
            if current_t < hit_distance && current_t > 0.0 {
                hit_distance = current_t;
                closest_sphere = Some(sphere);
            }
        }

        match closest_sphere {
            None => clear_color,
            Some(closest_sphere) => {
                let light_direction = vec3a(1., 1., -1.).normalize();

                let origin = ray.origin - closest_sphere.position;
                let hit_point = origin + ray.direction * hit_distance;

                let sphere_normal = hit_point.normalize();

                // cosine of the angle between hitpoin and the light direction
                // min light intenstiy is 0
                let light_intensity = sphere_normal.dot(-light_direction).max(0.05);

                closest_sphere.albedo * light_intensity
            }
        }
    }

    fn per_pixel(&self) {}

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
