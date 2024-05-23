use crate::buffers::{ObjectInfo, SceneTriangle};
use glam::{vec3a, Mat3A, Vec3A};
use std::fs::File;
use std::io::BufReader;

fn normalize_model(mut vectors: Vec<Vec3A>, rotation_matrix: Mat3A) -> Vec<Vec3A> {
    vectors = vectors
        .iter()
        .map(|&vector| rotation_matrix * vector)
        .collect::<Vec<_>>();

    let (min_coords, max_coords) = get_bounding_box(&vectors);

    let scale: f32 = 1.0 / min_coords.distance(max_coords);
    let translation = max_coords * scale;

    vectors
        .iter()
        .map(|&vector| vector * scale - translation)
        .collect::<Vec<_>>()
}

fn translate_model(vectors: Vec<Vec3A>, scale: f32, translation: Vec3A) -> Vec<Vec3A> {
    vectors
        .iter()
        .map(|&vector| vector * scale + translation)
        .collect::<Vec<_>>()
}

fn get_bounding_box(points: &Vec<Vec3A>) -> (Vec3A, Vec3A) {
    let (mut min_x, mut min_y, mut min_z) = (f32::MAX, f32::MAX, f32::MAX);
    let (mut max_x, mut max_y, mut max_z) = (f32::MIN, f32::MIN, f32::MIN);

    // Iterate through each coordinate array in the vector
    for &vec in points {
        if vec.x < min_x {
            min_x = vec.x;
        }
        if vec.x > max_x {
            max_x = vec.x;
        }

        if vec.y < min_y {
            min_y = vec.y;
        }
        if vec.y > max_y {
            max_y = vec.y;
        }

        if vec.z < min_z {
            min_z = vec.z;
        }
        if vec.z > max_z {
            max_z = vec.z;
        }
    }

    (vec3a(min_x, min_y, min_z), vec3a(max_x, max_y, max_z))
}

pub fn stl_triangles(
    filepath: &str,
    scale: f32,
    translation: Vec3A,
) -> (ObjectInfo, Vec<SceneTriangle>) {
    // Open the STL file
    let file = File::open(filepath).unwrap();
    let mut reader = BufReader::new(file);

    // Read the STL file
    let stl_file = stl_io::read_stl(&mut reader).expect("Failed to read STL file");

    // into vector of vec3a
    let original_points: Vec<Vec3A> = stl_file
        .vertices
        .iter()
        .map(|&vertex| vec3a(vertex[0], vertex[1], vertex[2]))
        .collect();

    let rotation_matrix = Mat3A::from_cols_slice(&[1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, 0.0]);

    let points = normalize_model(original_points, rotation_matrix);

    let translated_points = translate_model(points, scale, translation);

    let (min_coords, max_coords) = get_bounding_box(&translated_points);

    // Process the triangles
    let triangles: Vec<SceneTriangle> = stl_file
        .faces
        .iter()
        .map(|tri_index| {
            let indexes = tri_index.vertices;

            SceneTriangle::new(
                4,
                translated_points[indexes[0]].into(),
                translated_points[indexes[1]].into(),
                translated_points[indexes[2]].into(),
            )
        })
        .collect();

    let object_info = ObjectInfo {
        min_bounds: min_coords.into(),
        first_triangle_index: 0,
        max_bounds: max_coords.into(),
        triangle_count: triangles.len() as u32,
    };

    (object_info, triangles)
}
