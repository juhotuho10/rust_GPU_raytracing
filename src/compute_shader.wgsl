const F32_MAX: f32 = 3.4028235e+38;

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
    emission_color: vec3<f32>,            
    metallic: f32,            
    
    emission_power: f32,      
    
    // explicit padding to match 16 byte alignment
    _padding1: u32,
    _padding2: u32,
    _padding3: u32,             
}

struct SceneSphere {
    position: vec3<f32>,  
    radius: f32,         
    material_index: u32, 

     // explicit padding to match 16 byte alignment
    _padding1: u32,
    _padding2: u32,
    _padding3: u32,       
}


struct HitPayload {
    hit_distance: f32,
    world_position: vec3<f32>,
    world_normal: vec3<f32>,

    object_index: u32,
}

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
}




@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> camera_rays: array<vec3<f32>>;
@group(0) @binding(2) var<storage, read_write> output_data: array<u32>;
@group(0) @binding(3) var<uniform> ray_camera: RayCamera;
@group(0) @binding(4) var<uniform> material_array: array<SceneMaterial, 4>;
@group(0) @binding(5) var<uniform> sphere_array: array<SceneSphere, 4>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index: u32 =  (global_id.y * params.width) + global_id.x;

    let bounces: u32 = 4u;

    let f32_color: vec3<f32> = per_pixel(index, bounces);

    output_data[index] = pack_to_u32(f32_color);
    

    /*let ray_origin = ray_camera.origin;
    let ray_direction = camera_rays[index];
    let radius = 0.5;

    let a = dot(ray_direction, ray_direction);
    let b = dot(ray_origin, ray_direction) * 2.0;
    let c = dot(ray_origin, ray_origin) - (radius * radius);

    let discriminant = b * b - 4.0 * a * c;

    if discriminant > 0 {
        output_data[index] = pack_to_u32(vec3<f32>(1.0, 0.0, 1.0)); // red
    } else {
        output_data[index] = pack_to_u32(vec3<f32>(0.0, 0.0, 0.0)); // black
    }*/



    //if ray_camera.origin.z == 25.0 {
    //    output_data[index] = pack_to_u32(vec3<f32>(0.0, 1.0, 0.0)); // green
    //} else {
    //    output_data[index] = pack_to_u32(vec3<f32>(1.0, 0.0, 0.0)); // red
    //};



}

fn pack_to_u32(vector: vec3<f32>) -> u32 {
  // scale the f32 values from [0.0, 1.0] to [0.0, 255.0]
    let scaled_x: u32 = u32(vector.x * 255.0);
    let scaled_y: u32 = u32(vector.y * 255.0);
    let scaled_z: u32 = u32(vector.z * 255.0);

    // extract the least significant 8 bits (same as converting to u8)
    let byte0: u32 = scaled_x & 0xFFu;
    let byte1: u32 = scaled_y & 0xFFu;
    let byte2: u32 = scaled_z & 0xFFu;

    // pack the bits into a single u32 that will then be read as 4x u8 by the rendering pass
    return (byte0 << 0) | (byte1 << 8) | (byte2 << 16) | (255u << 24);
}

fn per_pixel(index: u32, bounces: u32) -> vec3<f32> {

    var ray = Ray( 
        ray_camera.origin,
        camera_rays[index]
    );

    
    var light_contribution = vec3<f32>(1.0, 1.0, 1.0);
    var light = vec3<f32>(0.0, 0.0, 0.0);
    let accumulation_index: u32 = 1u;
    //let skycolor = vec3<f32>(0., 0.04, 0.1);
    let skycolor = vec3<f32>(1.0, 0.0, 0.0);

    var seed: u32 = index * accumulation_index * 326624u;

    for (var i: u32 = 0u; i < bounces; i = i + 1) {

        let hit_payload: HitPayload = trace_ray(ray);

        if hit_payload.hit_distance < 0 {
            light += skycolor * light_contribution;
        }

        let hit_idex: u32 = hit_payload.object_index;
        let closest_sphere: SceneSphere = sphere_array[hit_idex];
        let material_index: u32 = closest_sphere.material_index;
        let current_material: SceneMaterial = material_array[material_index];

        light += current_material.emission_color * current_material.emission_power * light_contribution;

        light_contribution *= current_material.albedo * current_material.metallic;

        ray.origin = hit_payload.world_position + hit_payload.world_normal * 0.0001;

        ray.direction = normalize(hit_payload.world_normal + random_scaler(seed));

    }

    return light_contribution;

}


fn trace_ray(ray: Ray) -> HitPayload{
    // (bx^2 + by^2)t^2 + 2*(axbx + ayby)t + (ax^2 + by^2 - r^2) = 0
    // where
    // a = ray origin
    // b = ray direction
    // r = sphere radius
    // t = hit distance

    var hit_distance = F32_MAX;
    var closest_sphere_index: i32 = -1;

    let a: f32 = dot(ray.direction, ray.direction);

    // 4 used a a TEMPORARY sphere count, count should be passed in the params buffer
    for (var sphere_index: i32 = 0; sphere_index < 4; sphere_index = sphere_index + 1) {
        let sphere: SceneSphere = sphere_array[sphere_index];
        let origin: vec3<f32> = ray.origin - sphere.position;

        let b: f32 = 2.0 * dot(ray.direction, origin);
        let c: f32 = dot(origin, origin) - (sphere.radius * sphere.radius);

        // discriminant:
        // b^2 - 4*a*c
        let discriminant: f32 = b * b - 4. * a * c;

        if discriminant < 0.0 {
            // we missed the sphere
            continue;
        }

        // (-b +- discriminant) / 2a
        //let t0 = (-b + sqrt(discriminant)) / (2. * a);

        let current_t: f32 = (-b - sqrt(discriminant)) / (2. * a);

        if current_t > 0.0 && current_t < hit_distance {
            hit_distance = current_t;
            closest_sphere_index = sphere_index;
        }
        
    }

    if closest_sphere_index < 0 {
        return miss(ray);
    } else{
        return closest_hit(ray, hit_distance, closest_sphere_index);
    }

    
}

fn random_scaler(seed: u32) -> vec3<f32>{
    return vec3<f32>(-0.5, 0.9, 0.1);
}

fn miss(ray: Ray) -> HitPayload{
    return HitPayload(-1.0, 
    vec3<f32>(0.0, 0.0, 0.0),
    vec3<f32>(0.0, 0.0, 0.0),
    0u
    );
}

fn closest_hit(ray: Ray, hit_distance: f32, object_index: i32) -> HitPayload{
    let closest_sphere: SceneSphere = sphere_array[object_index];

    let hit_point: vec3<f32> = ray.origin + ray.direction * hit_distance;
    let sphere_normal: vec3<f32> = normalize(hit_point - closest_sphere.position);


    return HitPayload(hit_distance, 
    hit_point,
    sphere_normal,
    u32(object_index)
    );
}