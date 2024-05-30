use super::triangle_object::ObjectCreation;
use glam::vec3a;

use super::triangle_object::load_stl_files;

use super::buffers::{SceneMaterial, SceneSphere};

use super::renderer::RenderScene;

use super::image_texture::ImageTexture;

pub(crate) fn define_render_scene() -> RenderScene {
    // width and height for all images
    let texture_size = [400, 400];

    let shiny_green_texture = ImageTexture::new_from_color([1.0, 0.0, 0.0], texture_size); // 0
    let rough_blue_texture = ImageTexture::new_from_color([0.0, 0.6, 1.0], texture_size); // 2
    let glossy_pink_texture = ImageTexture::new_from_color([1.0, 0.1, 0.1], texture_size); // 2
    let shiny_orange_texture = ImageTexture::new_from_color([1.0, 0.7, 0.0], texture_size); // 3
    let cool_red_texture = ImageTexture::new_from_color([1.0, 0.0, 0.4], texture_size); // 4
    let shiny_white_texture = ImageTexture::new_from_color([1.0, 1.0, 1.0], texture_size); // 5

    // ###################### chess textures #####################################

    let b_queen_texture = ImageTexture::new_from_color([0.2, 0.2, 0.2], texture_size); // 6
    let b_king_texture = ImageTexture::new_from_color([0.2, 0.2, 0.2], texture_size); // 7
    let b_rook_texture = ImageTexture::new_from_color([0.2, 0.2, 0.2], texture_size); // 8
    let b_knight_texture = ImageTexture::new_from_color([0.2, 0.2, 0.2], texture_size); // 9
    let b_bishop_texture = ImageTexture::new_from_color([0.2, 0.2, 0.2], texture_size); // 10
    let b_pawns_texture = ImageTexture::new_from_color([0.2, 0.2, 0.2], texture_size); // 11

    let w_queen_texture = ImageTexture::new_from_color([1.0, 1.0, 1.0], texture_size); // 12
    let w_king_texture = ImageTexture::new_from_color([1.0, 1.0, 1.0], texture_size); // 13
    let w_rook_texture = ImageTexture::new_from_color([1.0, 1.0, 1.0], texture_size); // 14
    let w_knight_texture = ImageTexture::new_from_color([1.0, 1.0, 1.0], texture_size); // 15
    let w_bishop_texture = ImageTexture::new_from_color([1.0, 1.0, 1.0], texture_size); // 16
    let w_pawns_texture = ImageTexture::new_from_color([1.0, 1.0, 1.0], texture_size); // 17

    let chess_board_texture = ImageTexture::new_from_image("./textures/chess.png", texture_size); // 18

    // ###########################################################################

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

    // ###################### chess materials #####################################

    let b_queen_material = SceneMaterial {
        texture_index: 6,
        roughness: 0.1,
        emission_power: 0.0,
        specular: 0.2,
        specular_scatter: 0.2,
        glass: 0.0,
        refraction_index: 1.0,
        _padding: [0; 4],
    };
    let b_king_material = SceneMaterial {
        texture_index: 7,
        roughness: 0.1,
        emission_power: 0.0,
        specular: 0.2,
        specular_scatter: 0.2,
        glass: 0.0,
        refraction_index: 1.0,
        _padding: [0; 4],
    };
    let b_rook_material = SceneMaterial {
        texture_index: 8,
        roughness: 0.1,
        emission_power: 0.0,
        specular: 0.2,
        specular_scatter: 0.2,
        glass: 0.0,
        refraction_index: 1.0,
        _padding: [0; 4],
    };
    let b_knight_material = SceneMaterial {
        texture_index: 9,
        roughness: 0.1,
        emission_power: 0.0,
        specular: 0.2,
        specular_scatter: 0.2,
        glass: 0.0,
        refraction_index: 1.0,
        _padding: [0; 4],
    };
    let b_bishop_material = SceneMaterial {
        texture_index: 10,
        roughness: 0.1,
        emission_power: 0.0,
        specular: 0.2,
        specular_scatter: 0.2,
        glass: 0.0,
        refraction_index: 1.0,
        _padding: [0; 4],
    };
    let b_pawns_material = SceneMaterial {
        texture_index: 11,
        roughness: 0.1,
        emission_power: 0.0,
        specular: 0.2,
        specular_scatter: 0.2,
        glass: 0.0,
        refraction_index: 1.0,
        _padding: [0; 4],
    };
    let w_queen_material = SceneMaterial {
        texture_index: 12,
        roughness: 0.1,
        emission_power: 0.0,
        specular: 0.2,
        specular_scatter: 0.2,
        glass: 0.0,
        refraction_index: 1.0,
        _padding: [0; 4],
    };
    let w_king_material = SceneMaterial {
        texture_index: 13,
        roughness: 0.1,
        emission_power: 0.0,
        specular: 0.2,
        specular_scatter: 0.2,
        glass: 0.0,
        refraction_index: 1.0,
        _padding: [0; 4],
    };
    let w_rook_material = SceneMaterial {
        texture_index: 14,
        roughness: 0.1,
        emission_power: 0.0,
        specular: 0.2,
        specular_scatter: 0.2,
        glass: 0.0,
        refraction_index: 1.0,
        _padding: [0; 4],
    };
    let w_knight_material = SceneMaterial {
        texture_index: 15,
        roughness: 0.1,
        emission_power: 0.0,
        specular: 0.2,
        specular_scatter: 0.2,
        glass: 0.0,
        refraction_index: 1.0,
        _padding: [0; 4],
    };
    let w_bishop_material = SceneMaterial {
        texture_index: 16,
        roughness: 0.1,
        emission_power: 0.0,
        specular: 0.2,
        specular_scatter: 0.2,
        glass: 0.0,
        refraction_index: 1.0,
        _padding: [0; 4],
    };
    let w_pawns_material = SceneMaterial {
        texture_index: 17,
        roughness: 0.1,
        emission_power: 0.0,
        specular: 0.2,
        specular_scatter: 0.2,
        glass: 0.0,
        refraction_index: 1.0,
        _padding: [0; 4],
    };

    let chess_board_material = SceneMaterial {
        texture_index: 18,
        roughness: 0.8,
        emission_power: 0.0,
        specular: 0.2,
        specular_scatter: 0.5,
        glass: 0.0,
        refraction_index: 1.0,
        _padding: [0; 4],
    };

    // ###########################################################################

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

    let b_position = vec3a(5.3, -0.7, 0.0);
    let b_rotation = vec3a(90.0, 0.0, 0.0);

    let w_position = vec3a(-5.3, -0.7, 0.0);
    let w_rotation = vec3a(90.0, 180.0, 0.0);

    // offsets

    let tile = 1.51;

    let queen_offset = vec3a(0.0, 0.0, 0.5 * tile);
    let king_offset = vec3a(0.0, 0.0, -0.5 * tile);

    let rook_offset = vec3a(0.0, 0.0, 3.5 * tile);

    let knight_offset = vec3a(0.0, 0.0, 2.5 * tile);

    let bishop_offset = vec3a(0.0, 0.0, 1.5 * tile);

    let pawn1_offset = vec3a(-1.0 * tile, 0.0, 3.5 * tile);
    let pawn2_offset = vec3a(-1.0 * tile, 0.0, 2.5 * tile);
    let pawn3_offset = vec3a(-1.0 * tile, 0.0, 1.5 * tile);
    let pawn4_offset = vec3a(-1.0 * tile, 0.0, 0.5 * tile);
    let pawn5_offset = vec3a(-1.0 * tile, 0.0, -0.5 * tile);
    let pawn6_offset = vec3a(-1.0 * tile, 0.0, -1.5 * tile);
    let pawn7_offset = vec3a(-1.0 * tile, 0.0, -2.5 * tile);
    let pawn8_offset = vec3a(-1.0 * tile, 0.0, -3.5 * tile);

    let object_vec = load_stl_files(&[
        // ################# floor ####################
        ObjectCreation {
            file_path: "./3D_models/Wall.stl".to_string(),
            scale: 200.0,
            coordinates: vec3a(0.0, 7.066, 0.0),
            rotation: vec3a(0.0, 0.0, 0.0),
            material_index: 1,
        },
        // ###################### chess objects #####################################

        // ############### black pieces #######################
        ObjectCreation {
            file_path: "./3D_models/Queen.stl".to_string(),
            scale: 2.0,
            coordinates: b_position + queen_offset,
            rotation: b_rotation,
            material_index: 6,
        },
        ObjectCreation {
            file_path: "./3D_models/King.stl".to_string(),
            scale: 2.0,
            coordinates: b_position + king_offset,
            rotation: b_rotation,
            material_index: 7,
        },
        ObjectCreation {
            file_path: "./3D_models/Rook.stl".to_string(),
            scale: 2.0,
            coordinates: b_position + rook_offset,
            rotation: b_rotation,
            material_index: 8,
        },
        ObjectCreation {
            file_path: "./3D_models/Rook.stl".to_string(),
            scale: 2.0,
            coordinates: b_position - rook_offset,
            rotation: b_rotation,
            material_index: 8,
        },
        ObjectCreation {
            file_path: "./3D_models/Knight.stl".to_string(),
            scale: 2.0,
            coordinates: b_position + knight_offset,
            rotation: b_rotation,
            material_index: 9,
        },
        ObjectCreation {
            file_path: "./3D_models/Knight.stl".to_string(),
            scale: 2.0,
            coordinates: b_position - knight_offset,
            rotation: b_rotation,
            material_index: 9,
        },
        ObjectCreation {
            file_path: "./3D_models/Bishop.stl".to_string(),
            scale: 2.0,
            coordinates: b_position + bishop_offset,
            rotation: b_rotation,
            material_index: 10,
        },
        ObjectCreation {
            file_path: "./3D_models/Bishop.stl".to_string(),
            scale: 2.0,
            coordinates: b_position - bishop_offset,
            rotation: b_rotation,
            material_index: 10,
        },
        ObjectCreation {
            file_path: "./3D_models/Pawn.stl".to_string(),
            scale: 2.0,
            coordinates: b_position + pawn1_offset,
            rotation: b_rotation,
            material_index: 11,
        },
        ObjectCreation {
            file_path: "./3D_models/Pawn.stl".to_string(),
            scale: 2.0,
            coordinates: b_position + pawn2_offset,
            rotation: b_rotation,
            material_index: 11,
        },
        ObjectCreation {
            file_path: "./3D_models/Pawn.stl".to_string(),
            scale: 2.0,
            coordinates: b_position + pawn3_offset,
            rotation: b_rotation,
            material_index: 11,
        },
        ObjectCreation {
            file_path: "./3D_models/Pawn.stl".to_string(),
            scale: 2.0,
            coordinates: b_position + pawn4_offset,
            rotation: b_rotation,
            material_index: 11,
        },
        ObjectCreation {
            file_path: "./3D_models/Pawn.stl".to_string(),
            scale: 2.0,
            coordinates: b_position + pawn5_offset,
            rotation: b_rotation,
            material_index: 11,
        },
        ObjectCreation {
            file_path: "./3D_models/Pawn.stl".to_string(),
            scale: 2.0,
            coordinates: b_position + pawn6_offset,
            rotation: b_rotation,
            material_index: 11,
        },
        ObjectCreation {
            file_path: "./3D_models/Pawn.stl".to_string(),
            scale: 2.0,
            coordinates: b_position + pawn7_offset,
            rotation: b_rotation,
            material_index: 11,
        },
        ObjectCreation {
            file_path: "./3D_models/Pawn.stl".to_string(),
            scale: 2.0,
            coordinates: b_position + pawn8_offset,
            rotation: b_rotation,
            material_index: 11,
        },
        // ############### white pieces #######################
        ObjectCreation {
            file_path: "./3D_models/Queen.stl".to_string(),
            scale: 2.0,
            coordinates: w_position - queen_offset,
            rotation: w_rotation,
            material_index: 12,
        },
        ObjectCreation {
            file_path: "./3D_models/King.stl".to_string(),
            scale: 2.0,
            coordinates: w_position - king_offset,
            rotation: w_rotation,
            material_index: 13,
        },
        ObjectCreation {
            file_path: "./3D_models/Rook.stl".to_string(),
            scale: 2.0,
            coordinates: w_position + rook_offset,
            rotation: w_rotation,
            material_index: 14,
        },
        ObjectCreation {
            file_path: "./3D_models/Rook.stl".to_string(),
            scale: 2.0,
            coordinates: w_position - rook_offset,
            rotation: w_rotation,
            material_index: 14,
        },
        ObjectCreation {
            file_path: "./3D_models/Knight.stl".to_string(),
            scale: 2.0,
            coordinates: w_position + knight_offset,
            rotation: w_rotation,
            material_index: 15,
        },
        ObjectCreation {
            file_path: "./3D_models/Knight.stl".to_string(),
            scale: 2.0,
            coordinates: w_position - knight_offset,
            rotation: w_rotation,
            material_index: 15,
        },
        ObjectCreation {
            file_path: "./3D_models/Bishop.stl".to_string(),
            scale: 2.0,
            coordinates: w_position + bishop_offset,
            rotation: w_rotation,
            material_index: 16,
        },
        ObjectCreation {
            file_path: "./3D_models/Bishop.stl".to_string(),
            scale: 2.0,
            coordinates: w_position - bishop_offset,
            rotation: w_rotation,
            material_index: 16,
        },
        ObjectCreation {
            file_path: "./3D_models/Pawn.stl".to_string(),
            scale: 2.0,
            coordinates: w_position - pawn1_offset,
            rotation: w_rotation,
            material_index: 17,
        },
        ObjectCreation {
            file_path: "./3D_models/Pawn.stl".to_string(),
            scale: 2.0,
            coordinates: w_position - pawn2_offset,
            rotation: w_rotation,
            material_index: 17,
        },
        ObjectCreation {
            file_path: "./3D_models/Pawn.stl".to_string(),
            scale: 2.0,
            coordinates: w_position - pawn3_offset,
            rotation: w_rotation,
            material_index: 17,
        },
        ObjectCreation {
            file_path: "./3D_models/Pawn.stl".to_string(),
            scale: 2.0,
            coordinates: w_position - pawn4_offset,
            rotation: w_rotation,
            material_index: 17,
        },
        ObjectCreation {
            file_path: "./3D_models/Pawn.stl".to_string(),
            scale: 2.0,
            coordinates: w_position - pawn5_offset,
            rotation: w_rotation,
            material_index: 17,
        },
        ObjectCreation {
            file_path: "./3D_models/Pawn.stl".to_string(),
            scale: 2.0,
            coordinates: w_position - pawn6_offset,
            rotation: w_rotation,
            material_index: 17,
        },
        ObjectCreation {
            file_path: "./3D_models/Pawn.stl".to_string(),
            scale: 2.0,
            coordinates: w_position - pawn7_offset,
            rotation: w_rotation,
            material_index: 17,
        },
        ObjectCreation {
            file_path: "./3D_models/Pawn.stl".to_string(),
            scale: 2.0,
            coordinates: w_position - pawn8_offset,
            rotation: w_rotation,
            material_index: 17,
        },
        ObjectCreation {
            file_path: "./3D_models/Wall.stl".to_string(),
            scale: 20.0,
            coordinates: vec3a(0.0, 0.0, 0.0),
            rotation: vec3a(0.0, 90.0, 0.0),
            material_index: 18,
        },
        // ###########################################################################
    ]);

    RenderScene {
        image_textures: vec![
            shiny_green_texture,
            rough_blue_texture,
            glossy_pink_texture,
            shiny_orange_texture,
            cool_red_texture,
            shiny_white_texture,
            b_queen_texture,
            b_king_texture,
            b_rook_texture,
            b_knight_texture,
            b_bishop_texture,
            b_pawns_texture,
            w_queen_texture,
            w_king_texture,
            w_rook_texture,
            w_knight_texture,
            w_bishop_texture,
            w_pawns_texture,
            chess_board_texture,
        ],
        texture_size,

        materials: vec![
            shiny_green,
            rough_blue,
            glossy_pink,
            shiny_orange,
            cool_red,
            shiny_white,
            b_queen_material,
            b_king_material,
            b_rook_material,
            b_knight_material,
            b_bishop_material,
            b_pawns_material,
            w_queen_material,
            w_king_material,
            w_rook_material,
            w_knight_material,
            w_bishop_material,
            w_pawns_material,
            chess_board_material,
        ],
        spheres: vec![sphere_a, sphere_b, shiny_sphere],
        objects: object_vec,
        sky_color: [0., 0.04, 0.1],
    }
}
