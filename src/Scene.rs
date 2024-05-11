#[derive(Debug, Clone, Copy, PartialEq)]

pub struct Scene {
    pub spheres: Vec<Sphere>,
}

pub struct Sphere {
    pub position: Vec3A,
    pub radius: f32,
    pub albedo: Vec3A,
}
