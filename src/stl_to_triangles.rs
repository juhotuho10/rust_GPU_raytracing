use crate::buffers::{ObjectInfo, SceneTriangle};
use nalgebra::{Matrix3, Vector3};
use std::fs::File;
use std::io::BufReader;

fn rotate_vertex(vertex: [f32; 3], rotation_matrix: &Matrix3<f32>) -> [f32; 3] {
    let vector = Vector3::new(vertex[0], vertex[1], vertex[2]);
    let rotated_vector = rotation_matrix * vector;
    [rotated_vector.x, rotated_vector.y, rotated_vector.z]
}

pub fn stl_triangles(filepath: &str) -> (ObjectInfo, Vec<SceneTriangle>) {
    // Open the STL file
    let file = File::open(filepath).unwrap();
    let mut reader = BufReader::new(file);

    // Read the STL file
    let stl_file = stl_io::read_stl(&mut reader).expect("Failed to read STL file");

    let rotation_matrix = Matrix3::new(1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 1.0, 0.0);

    // into vector of arrays
    let mut points: Vec<[f32; 3]> = stl_file
        .vertices
        .iter()
        .map(|&values| values.into())
        .collect();

    // apply rotation
    points = points
        .iter()
        .map(|&vertex| rotate_vertex(vertex, &rotation_matrix))
        .collect();

    // scale and move the triangles
    points = points
        .iter()
        .map(|&vertex| [vertex[0] / 50.0, vertex[1] / 50.0 - 0.5, vertex[2] / 50.0])
        .collect();

    // Process the triangles
    let triangles: Vec<SceneTriangle> = stl_file
        .faces
        .iter()
        .map(|tri_index| {
            let indexes = tri_index.vertices;

            SceneTriangle::new(
                4,
                points[indexes[0]],
                points[indexes[1]],
                points[indexes[2]],
            )
        })
        .collect();

    let (mut min_x, mut min_y, mut min_z) = (f32::MAX, f32::MAX, f32::MAX);
    let (mut max_x, mut max_y, mut max_z) = (f32::MIN, f32::MIN, f32::MIN);

    // Iterate through each coordinate array in the vector
    for &[x, y, z] in &points {
        if x < min_x {
            min_x = x;
        }
        if x > max_x {
            max_x = x;
        }

        if y < min_y {
            min_y = y;
        }
        if y > max_y {
            max_y = y;
        }

        if z < min_z {
            min_z = z;
        }
        if z > max_z {
            max_z = z;
        }
    }

    let object_info = ObjectInfo {
        min_bounds: [min_x, min_y, min_z],
        first_triangle_index: 0,
        max_bounds: [max_x, max_y, max_z],
        triangle_count: triangles.len() as u32,
    };
    (object_info, triangles)
}
