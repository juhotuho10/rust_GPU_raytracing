use crate::buffers::{ObjectInfo, SceneTriangle};
use glam::{vec3a, Mat3A, Vec3A};
use std::fs::File;
use std::io::BufReader;

#[derive(Debug, Clone)]
pub struct SceneObject {
    normalized_points: Vec<Vec3A>,
    point_indexes: Vec<[usize; 3]>,
    pub rotation: Mat3A,
    pub scale: f32,
    pub transformation: Vec3A,
    pub center_location: Vec3A,
    pub object_info: ObjectInfo,
    pub object_triangles: Vec<SceneTriangle>,
}

impl SceneObject {
    pub fn new(filepath: &str, scale: f32, transformation: Vec3A) -> SceneObject {
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

        let rotation_matrix =
            Mat3A::from_cols_slice(&[1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, 0.0]);

        let points = normalize_model(original_points, rotation_matrix);

        let translated_points = translate_model(points, scale, transformation);

        let (mut min_coords, mut max_coords) = get_bounding_box(&translated_points);

        let (surface_points, surface_transformation) =
            move_model_to_surface(translated_points, max_coords);

        min_coords -= surface_transformation;
        max_coords -= surface_transformation;

        let total_transformation = transformation - surface_transformation;

        let point_indexes: Vec<[usize; 3]> = stl_file
            .faces
            .iter()
            .map(|vertex| vertex.vertices)
            .collect();

        // Process the triangles
        let triangles: Vec<SceneTriangle> = point_indexes
            .iter()
            .map(|indexes| {
                SceneTriangle::new(
                    4,
                    surface_points[indexes[0]].into(),
                    surface_points[indexes[1]].into(),
                    surface_points[indexes[2]].into(),
                )
            })
            .collect();

        let object_info = ObjectInfo {
            min_bounds: min_coords.into(),
            first_triangle_index: 0,
            max_bounds: max_coords.into(),
            triangle_count: triangles.len() as u32,
        };

        let center_location = (min_coords + max_coords) / 2.0;

        SceneObject {
            normalized_points: surface_points,
            point_indexes,
            scale: 1.0,
            rotation: rotation_matrix,
            transformation: total_transformation,
            center_location,
            object_info,
            object_triangles: triangles,
        }
    }
}

fn normalize_model(mut points: Vec<Vec3A>, rotation_matrix: Mat3A) -> Vec<Vec3A> {
    points = points
        .iter()
        .map(|&point| rotation_matrix * point)
        .collect::<Vec<_>>();

    let (min_coords, max_coords) = get_bounding_box(&points);

    let average = (min_coords + max_coords) / 2.0;

    let scale: f32 = 1.0 / min_coords.distance(max_coords);
    let transformation = average * scale;

    points
        .iter()
        .map(|&point| point * scale - transformation)
        .collect::<Vec<_>>()
}

fn translate_model(points: Vec<Vec3A>, scale: f32, transformation: Vec3A) -> Vec<Vec3A> {
    points
        .iter()
        .map(|&point| point * scale + transformation)
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

fn move_model_to_surface(mut points: Vec<Vec3A>, max_coords: Vec3A) -> (Vec<Vec3A>, Vec3A) {
    let transformation: Vec3A = max_coords * Vec3A::Y;

    points = points
        .iter()
        .map(|&point| point - transformation)
        .collect::<Vec<_>>();
    (points, transformation)
}
