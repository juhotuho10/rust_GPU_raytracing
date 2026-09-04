use crate::image_texture::ImageTexture;
use glam::Vec3A;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Params {
    pub screen_width: u32,       // float, aligned to 4 bytes
    pub screen_height: u32,      // float, aligned to 4 bytes
    pub accumulation_index: u32, // u32, aligned to 4 bytes
    pub accumulate: u32,         // u32, aligned to 4 bytes
    pub sphere_count: u32,       // u32, aligned to 4 bytes
    pub object_count: u32,       // u32, aligned to 4 bytes
    pub compute_per_frame: u32,  // u32, aligned to 4 bytes
    pub texture_width: u32,      // u32, aligned to 4 bytes
    pub texture_height: u32,     // u32, aligned to 4 bytes
    pub textue_count: u32,       // u32, aligned to 4 bytes
    pub env_map_width: u32,      // u32, aligned to 4 bytes
    pub env_map_height: u32,     // u32, aligned to 4 bytes
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RayCamera {
    pub origin: [f32; 3],  // vec3, aligned to 12 bytes
    pub _padding: [u8; 4], // padding to ensure 16-byte alignment
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Ray {
    pub direction: [f32; 3], // vec3, aligned to 12 bytes
    pub _padding: [u8; 4],   // padding to ensure 16-byte alignment
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SceneSphere {
    pub position: [f32; 3],  // vec3, aligned to 12 bytes
    pub radius: f32,         // f32, aligned to 4 bytes
    pub material_index: u32, // u32, aligned to 4 bytes
    pub _padding: [u8; 12],  // padding to ensure 16-byte alignment
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SceneMaterial {
    pub texture_index: u32,    // vec3, aligned to 12 bytes
    pub roughness: f32,        // f32, aligned to 4 bytes
    pub emission_power: f32,   // f32, aligned to 4 bytes
    pub specular: f32,         // f32, aligned to 4 bytes
    pub specular_scatter: f32, // f32, aligned to 4 bytes
    pub glass: f32,            // f32, aligned to 4 bytes
    pub refraction_index: f32, // f32, aligned to 4 bytes
    pub _padding: [u8; 4],     // padding to ensure 16-byte alignment
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ObjectInfo {
    pub min_bounds: [f32; 3],        // vec3, aligned to 12 bytes
    pub first_sub_object_index: u32, // f32, aligned to 4 bytes
    pub max_bounds: [f32; 3],        // vec3, aligned to 12 bytes
    pub sub_object_count: u32,       // f32, aligned to 4 bytes
    pub material_index: u32,         // f32, aligned to 4 bytes
    pub _padding: [u8; 12],          // padding to ensure 16-byte alignment
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SubObjectInfo {
    pub min_bounds: [f32; 3],      // vec3, aligned to 12 bytes
    pub first_triangle_index: u32, // f32, aligned to 4 bytes
    pub max_bounds: [f32; 3],      // vec3, aligned to 12 bytes
    pub triangle_count: u32,       // f32, aligned to 4 bytes
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SceneTriangle {
    pub a: [f32; 3],       //
    _padding: [u8; 4],     // padding to ensure 16-byte alignment
    pub edge_ab: [f32; 3], // vec3, aligned to 12 bytes
    _padding2: [u8; 4],    // padding to ensure 16-byte alignment
    pub edge_ac: [f32; 3], // vec3, aligned to 12 bytes
    _padding3: [u8; 4],    // padding to ensure 16-byte alignment
    calc_normal: [f32; 3], // vec3, aligned to 12 bytes
    _padding4: [u8; 4],    // padding to ensure 16-byte alignment
    face_normal: [f32; 3], // vec3, aligned to 12 bytes
    _padding5: [u8; 4],    // padding to ensure 16-byte alignment
}

impl SceneTriangle {
    pub fn new(a: Vec3A, b: Vec3A, c: Vec3A) -> SceneTriangle {
        // precalculations to save on compute

        let edge_ab = b - a;
        let edge_ac = c - a;

        let calc_normal = edge_ab.cross(edge_ac);
        let face_normal = calc_normal.normalize();

        SceneTriangle {
            a: a.into(),                     // vec3, aligned to 12 bytes
            _padding: [0; 4],                // padding to ensure 16-byte alignment
            edge_ab: edge_ab.into(),         // vec3, aligned to 12 bytes
            _padding2: [0; 4],               // padding to ensure 16-byte alignment
            edge_ac: edge_ac.into(),         // vec3, aligned to 12 bytes
            _padding3: [0; 4],               // padding to ensure 16-byte alignment
            calc_normal: calc_normal.into(), // vec3, aligned to 12 bytes
            _padding4: [0; 4],               // padding to ensure 16-byte alignment
            face_normal: face_normal.into(), // vec3, aligned to 12 bytes
            _padding5: [0; 4],               // padding to ensure 16-byte alignment
        }
    }
}

pub struct RenderScene {
    pub spheres: Vec<SceneSphere>,
    pub texture_size: [u32; 2],
    pub image_textures: Vec<ImageTexture>,
    pub materials: Vec<SceneMaterial>,
    pub objects: Vec<crate::triangle_object::SceneObject>,
    pub environment_map: ImageTexture,
    pub env_map_size: [u32; 2],
}

impl RenderScene {
    pub fn total_triangle_count(&self) -> usize {
        self.objects
            .iter()
            .map(|obj| obj.object_triangles.len())
            .sum()
    }

    pub fn total_sub_object_count(&self) -> usize {
        self.objects
            .iter()
            .map(|obj| obj.sub_object_info.len())
            .sum()
    }
}
