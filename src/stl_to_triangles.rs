use crate::buffers::SceneTriangle;
use nalgebra::{Matrix3, Vector3};
use std::fs::File;
use std::io::BufReader;

pub fn stl_triangles(filepath: &str) -> Vec<SceneTriangle> {
    // Open the STL file
    let file = File::open(filepath).unwrap();
    let mut reader = BufReader::new(file);

    // Read the STL file
    let stl_file = stl_io::read_stl(&mut reader).expect("Failed to read STL file");

    let rotation_matrix = Matrix3::new(1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 1.0, 0.0);

    let mut points: Vec<[f32; 3]> = stl_file
        .vertices
        .iter()
        .map(|&values| values.into())
        .collect();

    points = points
        .iter()
        .map(|&vertex| rotate_vertex(vertex, &rotation_matrix))
        .collect();

    points = points
        .iter()
        .map(|&vertex| [vertex[0] / 50.0, vertex[1] / 50.0 - 0.5, vertex[2] / 50.0])
        .collect();

    fn rotate_vertex(vertex: [f32; 3], rotation_matrix: &Matrix3<f32>) -> [f32; 3] {
        let vector = Vector3::new(vertex[0], vertex[1], vertex[2]);
        let rotated_vector = rotation_matrix * vector;
        [rotated_vector.x, rotated_vector.y, rotated_vector.z]
    }

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

    triangles
}
