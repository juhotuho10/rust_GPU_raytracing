use crate::buffers::{ObjectInfo, SceneTriangle, SubObjectInfo};
use glam::{Mat3A, Vec3A, vec3a};
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
    let mut sub_object_count = 0;
    let mut _triangle_count = 0;
    let mut scene_object_vec = vec![];

    for obj_data in object_data_vec {
        let mut new_obj = SceneObject::new(
            &obj_data.file_path,
            obj_data.scale,
            obj_data.coordinates,
            obj_data.rotation,
            obj_data.material_index,
        );

        (sub_object_count, _triangle_count) =
            new_obj.create_sub_objects(sub_object_count, _triangle_count);

        scene_object_vec.push(new_obj);
    }

    scene_object_vec
}

pub struct SceneObject {
    normalized_points: Vec<Vec3A>,
    point_indexes: Vec<[usize; 3]>,
    pub rotation: Vec3A,
    pub scale: f32,
    pub transformation: Vec3A,
    pub center_location: Vec3A,
    pub material_index: u32,
    pub object_info: ObjectInfo,
    pub sub_object_info: Vec<SubObjectInfo>,
    pub object_triangles: Vec<SceneTriangle>,
    n_sub_object_triangles: usize,
}

impl SceneObject {
    pub fn new(
        filepath: &str,
        scale: f32,
        transformation: Vec3A,
        rotation: Vec3A,
        material_index: u32,
    ) -> SceneObject {
        assert!(scale > 0.0, "scale has to be over 0.0");

        let file = File::open(filepath).expect("could not open STL file from path");
        let stl_file =
            stl_io::read_stl(&mut BufReader::new(file)).expect("Failed to read STL file");

        let original_points: Vec<Vec3A> = stl_file
            .vertices
            .iter()
            .map(|&v| vec3a(v[0], v[1], v[2]))
            .collect();

        let scaled_points = normalize_model(original_points, rotation, scale);

        let (min_coords, max_coords) = get_bounding_box(&scaled_points);

        // Lift the model so its lowest point sits at y = 0, then apply the world transform.
        let surface_transformation = -max_coords * Vec3A::Y;
        let total_transformation = surface_transformation + transformation;

        let point_indexes: Vec<[usize; 3]> =
            stl_file.faces.iter().map(|face| face.vertices).collect();

        let transformed_points =
            transform_points(&scaled_points, Mat3A::IDENTITY, 1.0, total_transformation);

        let min_coords = min_coords + total_transformation;
        let max_coords = max_coords + total_transformation;

        let object_info = ObjectInfo {
            min_bounds: min_coords.into(),
            first_sub_object_index: 0, // temp values
            max_bounds: max_coords.into(),
            sub_object_count: 0, // temp values
            material_index,
            _padding: [0; 12],
        };

        let triangles = generate_triangles(&point_indexes, &transformed_points);

        SceneObject {
            normalized_points: scaled_points,
            point_indexes,
            scale: 1.0,
            rotation: Vec3A::ZERO,
            transformation: total_transformation,
            center_location: (min_coords + max_coords) / 2.0,
            material_index,
            object_info,
            object_triangles: triangles,
            sub_object_info: vec![],
            n_sub_object_triangles: 7,
        }
    }

    pub fn update_triangles(&mut self) {
        let transformed_points = transform_points(
            &self.normalized_points,
            euler_rotation(self.rotation),
            self.scale,
            self.transformation,
        );

        let (min_coords, max_coords) = get_bounding_box(&transformed_points);

        self.center_location = (min_coords + max_coords) / 2.0;
        self.object_info.min_bounds = min_coords.into();
        self.object_info.max_bounds = max_coords.into();

        self.object_triangles = generate_triangles(&self.point_indexes, &transformed_points);
    }

    pub fn set_model_to_surface(&mut self) {
        self.transformation -= Vec3A::from(self.object_info.max_bounds) * Vec3A::Y;
    }

    pub fn reset_rotation(&mut self) {
        self.rotation = Vec3A::ZERO;
    }

    pub fn create_sub_objects(
        &mut self,
        starting_sub_object_index: u32,
        starting_triangle_index: u32,
    ) -> (u32, u32) {
        let mut triangle_index = starting_triangle_index;

        self.sub_object_info = self
            .object_triangles
            .chunks(self.n_sub_object_triangles)
            .map(|chunk| {
                let (min_bounds, max_bounds) = chunk_bounds(chunk);

                let sub_object = SubObjectInfo {
                    min_bounds: min_bounds.into(),
                    first_triangle_index: triangle_index,
                    max_bounds: max_bounds.into(),
                    triangle_count: chunk.len() as u32,
                };

                triangle_index += chunk.len() as u32;
                sub_object
            })
            .collect();

        self.object_info.first_sub_object_index = starting_sub_object_index;
        self.object_info.sub_object_count = self.sub_object_info.len() as u32;

        (
            starting_sub_object_index + self.object_info.sub_object_count,
            triangle_index,
        )
    }

    pub fn update_sub_objects(&mut self) {
        for (sub_object, (min_bounds, max_bounds)) in self.sub_object_info.iter_mut().zip(
            self.object_triangles
                .chunks(self.n_sub_object_triangles)
                .map(chunk_bounds),
        ) {
            sub_object.min_bounds = min_bounds.into();
            sub_object.max_bounds = max_bounds.into();
        }
    }
}

fn transform_points(
    points: &[Vec3A],
    rotation: Mat3A,
    scale: f32,
    translation: Vec3A,
) -> Vec<Vec3A> {
    // applies rotation, then uniform scale, then translation
    points
        .iter()
        .map(|&point| rotation * point * scale + translation)
        .collect()
}

fn euler_rotation(rotation: Vec3A) -> Mat3A {
    fn deg_to_rad(deg: f32) -> f32 {
        deg * (PI / 180.0)
    }

    Mat3A::from_rotation_z(deg_to_rad(rotation.z))
        * Mat3A::from_rotation_y(deg_to_rad(rotation.y))
        * Mat3A::from_rotation_x(deg_to_rad(rotation.x))
}

fn normalize_model(points: Vec<Vec3A>, rotation: Vec3A, scale: f32) -> Vec<Vec3A> {
    // normalizes a model to fit in a 1 sized unit box
    let rotated = transform_points(&points, euler_rotation(rotation), 1.0, Vec3A::ZERO);

    let (min_coords, max_coords) = get_bounding_box(&rotated);

    let scale = scale / min_coords.distance(max_coords);
    let offset = (min_coords + max_coords) * 0.5 * scale;

    rotated
        .iter()
        .map(|&point| point * scale - offset)
        .collect()
}

fn chunk_bounds(chunk: &[SceneTriangle]) -> (Vec3A, Vec3A) {
    let mut min = vec3a(f32::MAX, f32::MAX, f32::MAX);
    let mut max = vec3a(f32::MIN, f32::MIN, f32::MIN);

    for triangle in chunk {
        let a = Vec3A::from_array(triangle.a);
        let b = a + Vec3A::from_array(triangle.edge_ab);
        let c = a + Vec3A::from_array(triangle.edge_ac);

        min = min.min(a).min(b).min(c);
        max = max.max(a).max(b).max(c);
    }

    (min, max)
}

fn generate_triangles(
    point_indexes: &[[usize; 3]],
    transformed_points: &[Vec3A],
) -> Vec<SceneTriangle> {
    point_indexes
        .iter()
        .map(|&[a, b, c]| {
            SceneTriangle::new(
                transformed_points[a],
                transformed_points[b],
                transformed_points[c],
            )
        })
        .collect()
}

fn get_bounding_box(points: &[Vec3A]) -> (Vec3A, Vec3A) {
    let mut min = vec3a(f32::MAX, f32::MAX, f32::MAX);
    let mut max = vec3a(f32::MIN, f32::MIN, f32::MIN);

    for &point in points {
        min = min.min(point);
        max = max.max(point);
    }

    (min, max)
}
