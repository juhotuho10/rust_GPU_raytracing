use super::triangle_object::ObjectCreation;
use glam::vec3a;

use super::triangle_object::load_stl_files;

use super::buffers::{SceneMaterial, SceneSphere};

use super::renderer::RenderScene;

use super::image_texture::ImageTexture;

pub(crate) fn define_render_scene() -> RenderScene {
    // width and height for all images
    let texture_size = [400, 400];

    let shiny_green_texture = ImageTexture::new_from_color([1.0, 0.0, 0.0], texture_size);
    let rough_blue_texture = ImageTexture::new_from_image("./textures/chess.png", texture_size);
    let glossy_pink_texture = ImageTexture::new_from_color([1.0, 0.1, 0.1], texture_size);
    let shiny_orange_texture = ImageTexture::new_from_color([1.0, 0.7, 0.0], texture_size);
    let cool_red_texture = ImageTexture::new_from_color([1.0, 0.0, 0.4], texture_size);
    let shiny_white_texture = ImageTexture::new_from_color([1.0, 1.0, 1.0], texture_size);

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

    // ############# chess board ##############################

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
        texture_size,

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
