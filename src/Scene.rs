use glam::Vec3A;

#[derive(Debug, Clone, PartialEq)]
pub struct RenderScene {
    pub spheres: Vec<Sphere>,
    pub materials: Vec<Material>,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sphere {
    pub position: Vec3A,
    pub radius: f32,
    pub material_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Material {
    pub albedo: Vec3A,
    pub roughness: f32,
    pub metallic: f32,
}
