use core::time;

use egui::Context;
use egui_winit_platform::Platform;
use glam::{mat4, quat, vec2, vec3a, vec4, Mat4, Quat, Vec2, Vec3A, Vec4};
use wgpu::hal::auxil::db;

pub struct Camera {
    pub position: Vec3A,
    pub direction: Vec3A,
    pub ray_directions: Vec<Vec3A>,

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
            position: vec3a(0., 0., 3.),
            direction: vec3a(1., 0., 0.),
            ray_directions: vec![],

            viewport_width: width,
            viewport_height: height,

            near_clip: 0.1,
            far_clip: 100.0,
            vertical_fov: 45.0,

            movement_speed: 0.05,
            turning_speed: 0.003,

            projection: Mat4::from_cols_slice(&[1.0; 16]),
            inverse_projection: Mat4::from_cols_slice(&[1.0; 16]),
            view: Mat4::from_cols_slice(&[1.0; 16]),
            inverse_view: Mat4::from_cols_slice(&[1.0; 16]),
        };

        camera.on_resize(width, height);
        camera
    }

    pub fn on_update(&mut self, timestep: &f32, egui_context: &Context) -> bool {
        let up_direction = glam::Vec3A::Y;

        let right_direction = self.direction.cross(up_direction);

        let mut moved: bool = false;

        let mouse_delta = egui_context.input(|i: &egui::InputState| i.pointer.delta());

        egui_context.input(|input: &egui::InputState| {
            if input.key_down(egui::Key::W) {
                // holding W
                self.position += timestep * self.movement_speed * self.direction;
                moved = true;
            } else if input.key_down(egui::Key::S) {
                // holding S
                self.position -= timestep * self.movement_speed * self.direction;
                moved = true;
            }

            if input.key_down(egui::Key::D) {
                // holding D
                self.position += timestep * self.movement_speed * right_direction;
                moved = true;
            } else if input.key_down(egui::Key::A) {
                // holding A
                self.position -= timestep * self.movement_speed * right_direction;
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
            self.recalculate_ray_directions();
        }

        moved
    }

    fn recalculate_projection(&mut self) {
        let fov_rad: f32 = self.vertical_fov.to_radians();
        let aspect_ratio = (self.viewport_width / self.viewport_height) as f32;
        self.projection =
            Mat4::perspective_rh_gl(fov_rad, aspect_ratio, self.near_clip, self.far_clip);

        self.inverse_projection = self.projection.inverse();
    }

    pub fn recalculate_view(&mut self) {
        self.view = Mat4::look_at_rh(
            self.position.into(),
            (self.position + self.direction).into(),
            glam::Vec3::Y,
        );
        self.inverse_view = self.view.inverse();
    }

    pub fn recalculate_ray_directions(&mut self) {
        self.ray_directions.resize(
            (self.viewport_width * self.viewport_height) as usize,
            Vec3A::ZERO,
        );

        // converting normalized -1 to 1 directions into worldspace directions
        for y in 0..self.viewport_height {
            let y_coord = y as f32 / self.viewport_height as f32;
            for x in 0..self.viewport_width {
                let x_coord = x as f32 / self.viewport_width as f32;

                // normalized between -1 and 1
                let normalized_coord = vec2(x_coord, y_coord) * 2.0 - Vec2::ONE;

                let target: Vec4 = self.inverse_projection
                    * vec4(normalized_coord.x, normalized_coord.y, 1.0, 1.0);

                let target_vec3: Vec3A = target.truncate().into();

                let world_space_target: Vec4 = (target_vec3 / target.w).normalize().extend(0.0);

                let ray_direction: Vec3A =
                    (self.inverse_view * world_space_target).truncate().into();

                // caching the ray directions so we dont need to calculate them every frame
                self.ray_directions[(x + y * self.viewport_width) as usize] = ray_direction;
            }
        }
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
