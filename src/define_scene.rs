use glam::vec3a;

use super::triangle_object::ObjectCreation;

use super::triangle_object::load_stl_files;

use super::buffers::{SceneMaterial, SceneSphere};

use super::renderer::RenderScene;

use image::{ImageBuffer, Rgba};

#[derive(Debug, Clone)]
pub struct ImageTexture {
    pub from_color: bool,
    pub color: Option<[f32; 3]>,
    pub image_buffer: ImageBuffer<Rgba<u8>, Vec<u8>>,
}

impl ImageTexture {
    pub fn new_from_color(color: [f32; 3], width: u32, height: u32) -> ImageTexture {
        ImageTexture {
            from_color: true,
            color: Some(color),
            image_buffer: solid_color_image(color, width, height),
        }
    }

    pub fn new_from_image(path: &str) -> ImageTexture {
        ImageTexture {
            from_color: false,
            color: None,
            image_buffer: load_image(path),
        }
    }

    pub fn update_color(&mut self) {
        // we dont recolor textures that were loaded from files
        if let Some(color) = self.color {
            let rgba_color = [
                (color[0] * 255.0) as u8,
                (color[1] * 255.0) as u8,
                (color[2] * 255.0) as u8,
                255,
            ];
            for pixel in self.image_buffer.pixels_mut() {
                *pixel = Rgba(rgba_color);
            }
        }
    }
}

pub(crate) fn define_render_scene() -> RenderScene {
    let width = 100;
    let height = 100;

    let shiny_green_texture = ImageTexture::new_from_color([0.0, 0.8, 0.4], width, height);
    let rough_blue_texture = ImageTexture::new_from_color([0.3, 0.2, 0.8], width, height);
    let glossy_pink_texture = ImageTexture::new_from_color([1.0, 0.1, 1.0], width, height);
    let shiny_orange_texture = ImageTexture::new_from_color([1.0, 0.7, 0.0], width, height);
    let cool_red_texture = ImageTexture::new_from_color([1.0, 0.0, 0.4], width, height);
    let shiny_white_texture = ImageTexture::new_from_color([1.0, 1.0, 1.0], width, height);

    let shiny_green = SceneMaterial {
        texture_index: 0,
        roughness: 0.4,
        emission_power: 0.0,
        specular: 0.6,
        specular_scatter: 0.0,
        glass: 1.0,
        refraction_index: 2.0,
        _padding: [0; 4],
    };

    let rough_blue = SceneMaterial {
        texture_index: 1,
        roughness: 0.9,
        emission_power: 0.0,
        specular: 0.1,
        specular_scatter: 1.0,
        glass: 0.0,
        refraction_index: 1.0,
        _padding: [0; 4],
    };

    let glossy_pink = SceneMaterial {
        texture_index: 2,
        roughness: 0.7,
        emission_power: 0.0,
        specular: 0.5,
        specular_scatter: 0.1,
        glass: 0.0,
        refraction_index: 1.0,
        _padding: [0; 4],
    };

    let shiny_orange = SceneMaterial {
        texture_index: 3,
        roughness: 0.3,
        emission_power: 10.0,
        specular: 0.3,
        specular_scatter: 0.1,
        glass: 0.0,
        refraction_index: 1.0,
        _padding: [0; 4],
    };

    let cool_red = SceneMaterial {
        texture_index: 4,
        roughness: 0.9,
        emission_power: 0.0,
        specular: 0.5,
        specular_scatter: 0.4,
        glass: 1.0,
        refraction_index: 1.5,
        _padding: [0; 4],
    };

    let shiny_white = SceneMaterial {
        texture_index: 5,
        roughness: 0.7,
        emission_power: 0.0,
        specular: 0.5,
        specular_scatter: 0.1,
        glass: 1.0,
        refraction_index: 1.5,
        _padding: [0; 4],
    };

    let sphere_a: SceneSphere = SceneSphere {
        position: [1., -0.5, -2.],
        radius: 0.5,
        material_index: 2,
        _padding: [0; 12],
    };

    let sphere_b: SceneSphere = SceneSphere {
        position: [-3., -2.0, 3.],
        radius: 2.0,
        material_index: 0,
        _padding: [0; 12],
    };

    let shiny_sphere: SceneSphere = SceneSphere {
        position: [3., -15.0, -5.],
        radius: 7.0,
        material_index: 3,
        _padding: [0; 12],
    };

    let object_vec = load_stl_files(&[
        ObjectCreation {
            file_path: "./3D_models/Queen.stl".to_string(),
            scale: 2.0,
            coordinates: vec3a(3.0, 0.0, 3.0),
            rotation: vec3a(90.0, 0.0, 0.0),
            material_index: 4,
        },
        ObjectCreation {
            file_path: "./3D_models/Knight.stl".to_string(),
            scale: 1.0,
            coordinates: vec3a(2.0, 0.0, 2.0),
            rotation: vec3a(90.0, 0.0, 0.0),
            material_index: 5,
        },
        // ################# floor ####################
        ObjectCreation {
            file_path: "./3D_models/Wall.stl".to_string(),
            scale: 200.0,
            coordinates: vec3a(0.0, 7.066, 0.0),
            rotation: vec3a(0.0, 0.0, 0.0),
            material_index: 1,
        },
    ]);

    RenderScene {
        image_textures: vec![
            shiny_green_texture,
            rough_blue_texture,
            glossy_pink_texture,
            shiny_orange_texture,
            cool_red_texture,
            shiny_white_texture,
        ],

        materials: vec![
            shiny_green,
            rough_blue,
            glossy_pink,
            shiny_orange,
            cool_red,
            shiny_white,
        ],
        spheres: vec![sphere_a, sphere_b, shiny_sphere],
        objects: object_vec,
        sky_color: [0., 0.04, 0.1],
    }
}

fn solid_color_image(color: [f32; 3], width: u32, height: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::new(width, height);
    let rgba_color = [
        (color[0] * 255.0) as u8,
        (color[1] * 255.0) as u8,
        (color[2] * 255.0) as u8,
        255,
    ];
    for pixel in img.pixels_mut() {
        *pixel = Rgba(rgba_color);
    }
    img
}

fn load_image(path: &str) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let img = image::open(path).expect("could not load the image");
    img.to_rgba8()
}
