use glam::vec3a;

use super::triangle_object::ObjectCreation;

use super::triangle_object::load_stl_files;

use super::buffers::{SceneMaterial, SceneSphere};

use super::renderer::RenderScene;

pub(crate) fn define_render_scene() -> RenderScene {
    let shiny_green = SceneMaterial {
        albedo: [0.1, 0.8, 0.4],
        roughness: 0.4,
        emission_power: 0.0,
        specular: 0.6,
        specular_scatter: 0.0,
        glass: 1.0,
        refraction_index: 2.0,
        _padding: [0; 12],
    };

    let rough_blue = SceneMaterial {
        albedo: [0.3, 0.2, 0.8],
        roughness: 0.9,
        emission_power: 0.0,
        specular: 0.1,
        specular_scatter: 1.0,
        glass: 0.0,
        refraction_index: 1.0,
        _padding: [0; 12],
    };

    let glossy_pink = SceneMaterial {
        albedo: [1.0, 0.1, 1.0],
        roughness: 0.7,
        emission_power: 0.0,
        specular: 0.5,
        specular_scatter: 0.1,
        glass: 0.0,
        refraction_index: 1.0,
        _padding: [0; 12],
    };

    let shiny_orange = SceneMaterial {
        albedo: [1.0, 0.7, 0.0],
        roughness: 0.3,
        emission_power: 10.0,
        specular: 0.3,
        specular_scatter: 0.1,
        glass: 0.0,
        refraction_index: 1.0,
        _padding: [0; 12],
    };

    let cool_red = SceneMaterial {
        albedo: [1.0, 0.0, 0.4],
        roughness: 0.9,
        emission_power: 0.0,
        specular: 0.5,
        specular_scatter: 0.4,
        glass: 1.0,
        refraction_index: 1.5,
        _padding: [0; 12],
    };

    let shiny_white = SceneMaterial {
        albedo: [1.0, 1.0, 1.0],
        roughness: 0.7,
        emission_power: 0.0,
        specular: 0.5,
        specular_scatter: 0.1,
        glass: 1.0,
        refraction_index: 1.5,
        _padding: [0; 12],
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
