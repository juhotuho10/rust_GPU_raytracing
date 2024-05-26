use crate::buffers::{ObjectInfo, SceneTriangle};
use glam::{vec3a, Mat3A, Vec3A};
use std::fs::File;
use std::io::BufReader;

use std::f32::consts::PI;

pub struct ObjectCreation {
    pub file_path: String,
    pub scale: f32,
    pub coordinates: Vec3A,
    pub rotation: Vec3A,
    pub material_index: u32,
}

pub fn load_stl_files(object_data_vec: &[ObjectCreation]) -> Vec<SceneObject> {
    let mut triangle_count = 0;
    let mut scene_object_vec = vec![];

    for obj_data in object_data_vec {
        let new_obj = SceneObject::new(
            &obj_data.file_path,
            obj_data.scale,
            obj_data.coordinates,
            obj_data.rotation,
            obj_data.material_index,
            triangle_count,
        );

        triangle_count += new_obj.object_triangles.len() as u32;
        scene_object_vec.push(new_obj);
    }

    scene_object_vec
}

#[derive(Debug, Clone)]
pub struct SceneObject {
    normalized_points: Vec<Vec3A>,
    point_indexes: Vec<[usize; 3]>,
    pub rotation: Vec3A,
    pub scale: f32,
    pub transformation: Vec3A,
    pub center_location: Vec3A,
    pub material_index: u32,
    pub object_info: ObjectInfo,
    pub object_triangles: Vec<SceneTriangle>,
}

impl SceneObject {
    pub fn new(
        filepath: &str,
        scale: f32,
        transformation: Vec3A,
        rotation: Vec3A,
        material_index: u32,
        starting_triangle_index: u32,
    ) -> SceneObject {
        // Open the STL file
        let file = File::open(filepath).unwrap();
        let mut reader = BufReader::new(file);

        assert!(scale > 0.0, "scale has to be over 0.0");

        // Read the STL file
        let stl_file = stl_io::read_stl(&mut reader).expect("Failed to read STL file");

        // into vector of vec3a
        let original_points: Vec<Vec3A> = stl_file
            .vertices
            .iter()
            .map(|&vertex| vec3a(vertex[0], vertex[1], vertex[2]))
            .collect();

        let points = normalize_model(original_points, rotation);

        let scaled_points = scale_model(points, scale);

        let (mut min_coords, mut max_coords) = get_bounding_box(&scaled_points);

        let (surface_points, surface_transformation) =
            transform_points_to_surface(scaled_points.clone(), max_coords);

        let transformed_points = transform_model(surface_points, transformation);

        min_coords += surface_transformation + transformation;
        max_coords += surface_transformation + transformation;

        let total_transformation = transformation + surface_transformation;

        let point_indexes: Vec<[usize; 3]> = stl_file
            .faces
            .iter()
            .map(|vertex| vertex.vertices)
            .collect();

        // Process the triangles
        let triangles = generate_triangles(&point_indexes, &transformed_points);

        let object_info = ObjectInfo {
            min_bounds: min_coords.into(),
            first_triangle_index: starting_triangle_index,
            max_bounds: max_coords.into(),
            triangle_count: triangles.len() as u32,
            material_index,
            _padding: [0; 12],
        };

        let center_location = (min_coords + max_coords) / 2.0;

        SceneObject {
            normalized_points: scaled_points,
            point_indexes,
            scale: 1.0,
            rotation: Vec3A::ZERO,
            transformation: total_transformation,
            center_location,
            material_index,
            object_info,
            object_triangles: triangles,
        }
    }

    pub fn update_triangles(&mut self) {
        let rotated_points = rotate_to_angle(self.normalized_points.clone(), self.rotation);

        let scaled_points: Vec<Vec3A> = scale_model(rotated_points, self.scale);

        let transformed_points = transform_model(scaled_points, self.transformation);

        let (min_coords, max_coords) = get_bounding_box(&transformed_points);

        self.center_location = (min_coords + max_coords) / 2.0;

        self.object_info.min_bounds = min_coords.into();
        self.object_info.max_bounds = max_coords.into();

        let triangles: Vec<SceneTriangle> =
            generate_triangles(&self.point_indexes, &transformed_points);

        self.object_triangles = triangles;
    }

    pub fn set_model_to_surface(&mut self) {
        let transformation: Vec3A = self.object_info.max_bounds.into();
        let y_transform = transformation * Vec3A::Y;

        self.transformation -= y_transform;
    }

    pub fn reset_rotation(&mut self) {
        self.rotation = Vec3A::ZERO;
    }
}

fn generate_triangles(
    point_indexes: &[[usize; 3]],
    transformed_points: &[Vec3A],
) -> Vec<SceneTriangle> {
    let triangles: Vec<SceneTriangle> = point_indexes
        .iter()
        .map(|indexes| {
            SceneTriangle::new(
                transformed_points[indexes[0]],
                transformed_points[indexes[1]],
                transformed_points[indexes[2]],
            )
        })
        .collect();
    triangles
}

fn normalize_model(mut points: Vec<Vec3A>, rotation_matrix: Vec3A) -> Vec<Vec3A> {
    points = rotate_to_angle(points, rotation_matrix);

    let (min_coords, max_coords) = get_bounding_box(&points);

    let average = (min_coords + max_coords) / 2.0;

    let scale: f32 = 1.0 / min_coords.distance(max_coords);
    let transformation = average * scale;

    points
        .iter()
        .map(|&point| point * scale - transformation)
        .collect::<Vec<_>>()
}

fn rotate_to_angle(points: Vec<Vec3A>, rotation: Vec3A) -> Vec<Vec3A> {
    fn deg_to_rad(deg: f32) -> f32 {
        deg * (PI / 180.0)
    }

    let x_rad = deg_to_rad(rotation.x);
    let y_rad = deg_to_rad(rotation.y);
    let z_rad = deg_to_rad(rotation.z);

    let rotation_x = Mat3A::from_rotation_x(x_rad);
    let rotation_y = Mat3A::from_rotation_y(y_rad);
    let rotation_z = Mat3A::from_rotation_z(z_rad);

    let rotation = rotation_z * rotation_y * rotation_x;

    apply_rotation_matrix(points, rotation)
}

fn apply_rotation_matrix(points: Vec<Vec3A>, rotation: Mat3A) -> Vec<Vec3A> {
    points
        .iter()
        .map(|&point| rotation * point)
        .collect::<Vec<_>>()
}

fn scale_model(points: Vec<Vec3A>, scale: f32) -> Vec<Vec3A> {
    points
        .iter()
        .map(|&point| point * scale)
        .collect::<Vec<_>>()
}

fn transform_model(points: Vec<Vec3A>, transformation: Vec3A) -> Vec<Vec3A> {
    points
        .iter()
        .map(|&point| point + transformation)
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

fn transform_points_to_surface(mut points: Vec<Vec3A>, max_coords: Vec3A) -> (Vec<Vec3A>, Vec3A) {
    let transformation: Vec3A = -max_coords * Vec3A::Y;

    points = points
        .iter()
        .map(|&point| point + transformation)
        .collect::<Vec<_>>();
    (points, transformation)
}
