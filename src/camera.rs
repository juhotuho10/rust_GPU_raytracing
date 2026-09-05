use super::buffers::Ray;
use bytemuck::Zeroable;
use egui::Context;
use glam::{
    Mat4, Quat, Vec3A,
    camera::rh::{proj, view},
    vec3a,
};

use rayon::prelude::*;
#[derive(Debug, Clone, PartialEq)]
pub struct Camera {
    pub position: Vec3A,
    pub direction: Vec3A,

    near_clip: f32,
    far_clip: f32,
    vertical_fov: f32,

    pub viewport_width: u32,
    pub viewport_height: u32,

    pub movement_speed: f32,
    turning_speed: f32,

    projection: Mat4,
    inverse_projection: Mat4,
    view: Mat4,
    inverse_view: Mat4,
}

impl Camera {
    pub fn new(width: u32, height: u32) -> Camera {
        let mut camera = Camera {
            position: vec3a(0.0, -6.0, 25.),
            direction: vec3a(0., 0., -1.),

            viewport_width: width,
            viewport_height: height,

            near_clip: 0.1,
            far_clip: 100.0,
            vertical_fov: 45.0,

            movement_speed: 1.2,
            turning_speed: 0.001,

            projection: Mat4::from_cols_slice(&[1.0; 16]),
            inverse_projection: Mat4::from_cols_slice(&[1.0; 16]),
            view: Mat4::from_cols_slice(&[1.0; 16]),
            inverse_view: Mat4::from_cols_slice(&[1.0; 16]),
        };

        camera.recalculate_view();
        camera.recalculate_projection();
        camera.recalculate_ray_directions();

        camera
    }

    pub fn on_update(&mut self, mouse_delta: egui::Vec2, egui_context: &Context) -> bool {
        let up_direction = glam::Vec3A::Y;

        let right_direction = self.direction.cross(up_direction);

        let mut moved: bool = false;

        //let mouse_delta = egui_context.input(|i: &egui::InputState| i.pointer.delta());

        egui_context.input(|input: &egui::InputState| {
            // forward - backward
            if input.key_down(egui::Key::W) {
                // holding W
                self.position += self.movement_speed * self.direction;
                moved = true;
            } else if input.key_down(egui::Key::S) {
                // holding S
                self.position -= self.movement_speed * self.direction;
                moved = true;
            }

            // left - right
            if input.key_down(egui::Key::D) {
                // holding D
                self.position += self.movement_speed * right_direction;
                moved = true;
            } else if input.key_down(egui::Key::A) {
                // holding A
                self.position -= self.movement_speed * right_direction;
                moved = true;
            }

            // up - down
            if input.key_down(egui::Key::Q) {
                // holding Q
                self.position += self.movement_speed * up_direction;
                moved = true;
            } else if input.key_down(egui::Key::E) {
                // holding E
                self.position -= self.movement_speed * up_direction;
                moved = true;
            }
        });

        if mouse_delta != egui::Vec2::ZERO {
            // rotate the camera

            let pitch_delta: f32 = mouse_delta.y * self.turning_speed;
            let yaw_delta: f32 = mouse_delta.x * self.turning_speed;

            let right_rotation = Quat::from_axis_angle(right_direction.into(), pitch_delta);
            let up_rotation = Quat::from_axis_angle(up_direction.into(), -yaw_delta);

            let q: Quat = (right_rotation * up_rotation).normalize();
            self.direction = q.mul_vec3(self.direction.into()).into();

            moved = true;
        }

        if moved {
            self.recalculate_view();
        }
        moved
    }

    fn recalculate_projection(&mut self) {
        let fov_rad: f32 = self.vertical_fov.to_radians();
        let aspect_ratio = (self.viewport_width / self.viewport_height) as f32;
        self.projection =
            proj::opengl::perspective(fov_rad, aspect_ratio, self.near_clip, self.far_clip);

        self.inverse_projection = self.projection.inverse();
    }

    pub fn recalculate_view(&mut self) {
        self.view = view::look_at_mat4(
            self.position.into(),
            (self.position + self.direction).into(),
            glam::Vec3::Y,
        );
        self.inverse_view = self.view.inverse();
    }

    pub fn recalculate_ray_directions(&self) -> Vec<Ray> {
        let forward = self.direction.normalize_or_zero();

        let right_hat = forward.cross(glam::Vec3A::Y);
        let right_hat = if right_hat.length_squared() > 1e-8 {
            right_hat.normalize()
        } else {
            forward.any_orthogonal_vector()
        };
        let up_hat = right_hat.cross(forward);

        let tan_half_fov = (self.vertical_fov * 0.5).to_radians().tan();
        let step = 2.0 * tan_half_fov / self.viewport_height as f32;

        let width = self.viewport_width as usize;
        let height = self.viewport_height as usize;
        let mut new_ray_directions: Vec<Ray> = vec![Ray::zeroed(); width * height];

        let right_step = right_hat * step;
        let up_step = up_hat * step;
        let origin =
            forward - (right_step * (width as f32 * 0.5) + up_step * (height as f32 * 0.5));

        new_ray_directions
            .par_chunks_mut(width)
            .enumerate()
            .for_each(|(y, row)| {
                let row_base = origin + y as f32 * up_step;

                for (x, ray) in row.iter_mut().enumerate() {
                    let dir = row_base + x as f32 * right_step;

                    *ray = Ray {
                        direction: dir.normalize_or_zero().into(),
                        _padding: [0; 4],
                    };
                }
            });

        new_ray_directions
    }
    pub fn on_resize(&mut self, width: u32, height: u32) {
        if width == self.viewport_width && height == self.viewport_height {
            return;
        }

        self.viewport_width = width;
        self.viewport_height = height;

        self.recalculate_projection();
        self.recalculate_ray_directions();
    }
}
