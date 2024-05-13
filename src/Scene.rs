use glam::Vec3A;

#[derive(Debug, Clone, PartialEq)]
pub struct RenderScene {
    pub spheres: Vec<Sphere>,
    pub materials: Vec<Material>,
    pub sky_color: Vec3A,
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
    pub emission_color: Vec3A,
    pub emission_power: f32,
}

impl Material {
    pub fn get_emission(&self) -> Vec3A {
        self.emission_color * self.emission_power
    }
}
