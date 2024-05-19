struct Params {
    width: u32,

    // explicit padding to match 16 byte alignment
    _padding1: u32,
    _padding2: u32,
    _padding3: u32,
};


struct RayCamera {
    origin: vec3<f32>,    
    direction: vec3<f32>,  
};

struct SceneMaterial {
    albedo: vec3<f32>,         
    roughness: f32,           
    metallic: f32,            
    emission_color: vec3<f32>, 
    emission_power: f32,      
    
     // explicit padding to match 16 byte alignment
    _padding1: u32,           
}


struct SceneSphere {
    position: vec3<f32>,  
    radius: f32,         
    material_index: u32, 

     // explicit padding to match 16 byte alignment
    _padding1: u32,
    _padding2: u32,      
}



@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> camera_rays: array<vec3<f32>>;
@group(0) @binding(2) var<storage, read_write> output_data: array<u32>;
@group(0) @binding(3) var<uniform> ray_camera: RayCamera;
@group(0) @binding(4) var<uniform> material_array: array<SceneMaterial, 4>;
@group(0) @binding(5) var<uniform> sphere_array: array<SceneSphere, 4>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index =  (global_id.y * params.width) + global_id.x;

    

    let ray_origin = ray_camera.origin;
    let ray_direction = camera_rays[index];
    let radius = 0.5;

    let a = dot(ray_direction, ray_direction);
    let b = dot(ray_origin, ray_direction) * 2.0;
    let c = dot(ray_origin, ray_origin) - (radius * radius);

    let discriminant = b * b - 4.0 * a * c;

    if discriminant > 0 {
        output_data[index] = pack_to_u32(1.0, 0.0, 1.0); // red
    } else {
        output_data[index] = pack_to_u32(0.0, 0.0, 0.0); // black
    }

}

fn pack_to_u32(x: f32, y: f32, z: f32) -> u32 {
  // scale the f32 values from [0.0, 1.0] to [0.0, 255.0]
    let scaled_x: u32 = u32(x * 255.0);
    let scaled_y: u32 = u32(y * 255.0);
    let scaled_z: u32 = u32(z * 255.0);

    // extract the least significant 8 bits (same as converting to u8)
    let byte0: u32 = scaled_x & 0xFFu;
    let byte1: u32 = scaled_y & 0xFFu;
    let byte2: u32 = scaled_z & 0xFFu;

    // pack the bits into a single u32 that will then be read as 4x u8 by the rendering pass
    return (byte0 << 0) | (byte1 << 8) | (byte2 << 16) | (255u << 24);
}