const F32_MAX: f32 = 3.4028235e+38;
const U32_MAX: u32 = 4294967295u;

struct Params {
    sky_color: vec3<f32>,
    width: u32,
    accumulation_index: u32,
    accumulate: u32,
    // explicit padding to match 16 byte alignment
    _padding1: u32,
    _padding2: u32,
};


struct RayCamera {
    origin: vec3<f32>,    

    // explicit padding to match 16 byte alignment
     _padding1: u32,
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

struct SceneTriangle {
    a: vec3<f32>,
    material_index: u32,
    b: vec3<f32>,
     _padding1: u32,
    c: vec3<f32>,
    _padding2: u32,
    normal: vec3<f32>,
    _padding3: u32,
    // explicit padding to match 16 byte alignment
    // explicit padding to match 16 byte alignment
   
    
    // explicit padding to match 16 byte alignment   
   
    
}

struct HitPayload {
    hit_distance: f32,
    world_position: vec3<f32>,
    world_normal: vec3<f32>,

    material_index: u32,
}

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
}


@group(0) @binding(0) var<storage, read> params: Params;
@group(0) @binding(1) var<storage, read> camera_rays: array<vec3<f32>>;
@group(0) @binding(2) var<storage, read_write> output_data: array<u32>;
@group(0) @binding(3) var<uniform> ray_camera: RayCamera;
@group(0) @binding(4) var<uniform> material_array: array<SceneMaterial, 5>;
@group(0) @binding(5) var<uniform> sphere_array: array<SceneSphere, 4>;
@group(0) @binding(6) var<storage, read_write> accumulation_data: array<vec3<f32>>;
@group(0) @binding(7) var<uniform> triangle_array: array<SceneTriangle, 202>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index: u32 =  (global_id.y * params.width) + global_id.x;

    let bounces: u32 = 10u;

    var render_color = vec3<f32>(0.0);

    var random_index = params.accumulation_index;

    var pixel_color = accumulation_data[index];

    let renders_times = 2u;

    if params.accumulate == 1{

        for (var i: u32 = 0u; i < renders_times; i = i + 1) {
            pixel_color += per_pixel(index, bounces, random_index);
            random_index = random_index + 1;
        }
        accumulation_data[index] = pixel_color;

        var accumulated_color = pixel_color / f32(params.accumulation_index * renders_times);

        render_color = clamp(accumulated_color, vec3<f32>(0.0), vec3<f32>(1.0));
        

    }else{

        let f32_color: vec3<f32> = per_pixel(index, bounces, random_index);
        render_color = clamp(f32_color, vec3<f32>(0.0), vec3<f32>(1.0));
    }
    
    // pack 4 f32 values into a single u32 (4x u8 rgba color)
    output_data[index] = pack_to_u32(render_color);


    /*let f32_color: vec3<f32> = per_pixel(index, bounces);

    if (f32_color.x > 1.0 || f32_color.y > 1.0 || f32_color.z > 1.0 ) {
        output_data[index] = pack_to_u32(vec3<f32>(0.0, 1.0, 0.0)); // green
    } else {
        output_data[index] = pack_to_u32(vec3<f32>(1.0, 0.0, 0.0)); // red
    };*/

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

fn per_pixel(index: u32, bounces: u32, random_index: u32) -> vec3<f32> {

    var ray = Ray( 
        ray_camera.origin,
        camera_rays[index]
    );

    var seed: u32 = index * random_index * 326624u;

    ray.direction += random_scaler(&seed) * 0.001;

    
    var light_contribution = vec3<f32>(1.0);
    var light = vec3<f32>(0.0);

    

    for (var i: u32 = 0u; i < bounces; i = i + 1) {

        let hit_payload: HitPayload = trace_ray(ray);

        if hit_payload.hit_distance == F32_MAX {
            // we hit the sky
            light += params.sky_color * light_contribution;
            break;
        }

        let material_index: u32 = hit_payload.material_index;
        let current_material: SceneMaterial = material_array[material_index];

        let emitted_light = current_material.emission_color * current_material.emission_power;
        light += emitted_light * light_contribution;

        light_contribution *= current_material.albedo * current_material.metallic;

        


        // combination of ray math that worked well with triangles and spheres, 
        // the triangle world normal calculation somehow doesnt want to work properly
        ray.origin = ray.origin + hit_payload.hit_distance * ray.direction + hit_payload.world_normal * 0.0001;

        //ray.origin = hit_payload.world_position + hit_payload.world_normal * 0.0001;

        //ray.origin = hit_payload.world_position - ray.direction * 0.01;

   
        ray.direction = normalize(hit_payload.world_normal + random_normal_scaler(&seed));

    }

    
    return light;

}


fn trace_ray(ray: Ray) -> HitPayload{

    let sphere_hit_payload = check_spheres(ray);
    let triangle_hit_payload = check_triangles(ray);

    if sphere_hit_payload.hit_distance < triangle_hit_payload.hit_distance{
        return sphere_hit_payload;
    }else{
        return triangle_hit_payload;
    };
    
}

fn check_spheres(ray: Ray) -> HitPayload{

    // (bx^2 + by^2)t^2 + 2*(axbx + ayby)t + (ax^2 + by^2 - r^2) = 0
    // where
    // a = ray origin
    // b = ray direction
    // r = sphere radius
    // t = hit distance

    var closest_distance = F32_MAX;
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

        if (current_t > 0.0) && (current_t < closest_distance) {
            closest_distance = current_t;
            closest_sphere_index = sphere_index;
        }
        
    }

    if closest_sphere_index < 0 {
        return miss();
    } else{
        return sphere_hit(ray, closest_distance, u32(closest_sphere_index));
    }

}

fn check_triangles(ray: Ray) -> HitPayload{

    var closest_distance = F32_MAX;
    var closest_hitpayload: HitPayload = miss();

    let a: f32 = dot(ray.direction, ray.direction);

    for (var triangle_index: i32 = 0; triangle_index < 202; triangle_index = triangle_index + 1) {
        let tri: SceneTriangle = triangle_array[triangle_index];

        let edge_ab: vec3<f32> = tri.b - tri.a;
        let edge_ac: vec3<f32> = tri.c - tri.a;

        let normal: vec3<f32> = cross(edge_ab, edge_ac);
        
        let ao: vec3<f32> = ray.origin - tri.a; 
        let dao: vec3<f32> = cross(ao, ray.direction); 

        let determinant: f32 = -dot(ray.direction, normal);

        if determinant < 1.0e-6 {
            continue;
        }

        let inv_det: f32 = 1 / determinant;

        // calculate distance and intersection

        let distance: f32 = dot(ao, normal) * inv_det;

        if distance < 0.0 || distance > closest_distance {
            continue;
        }

        let u: f32 = dot(edge_ac, dao) * inv_det;

        if u < 0.0 {
            continue;
        }
        let v: f32 = -dot(edge_ab, dao) * inv_det;

        if v < 0.0 {
            continue;
        }
        let w: f32 = 1 - u - v;

        if w < 0.0 {
            continue;
        }

        var face_normal: vec3<f32> = normalize(normal);

        if determinant < 0.0 {
            face_normal = -face_normal;
        }


        closest_distance = distance;


        closest_hitpayload = HitPayload(
            distance,
            ray.origin * ray.direction * distance,
            face_normal,
            tri.material_index,

        );
  
    };

    return closest_hitpayload;

}

fn miss() -> HitPayload{
    return HitPayload(F32_MAX, 
    vec3<f32>(0.0),
    vec3<f32>(0.0),
    0u
    );
}


fn sphere_hit(ray: Ray, hit_distance: f32, object_index: u32) -> HitPayload{
    let closest_sphere: SceneSphere = sphere_array[object_index];

    let hit_point: vec3<f32> = ray.origin + ray.direction * hit_distance;
    let sphere_normal: vec3<f32> = normalize(hit_point - closest_sphere.position);


    return HitPayload(hit_distance, 
    hit_point,
    sphere_normal,
    closest_sphere.material_index,
    );
}


fn pcg_hash(seed: ptr<function, u32>) -> f32 {
    // random float between 0 and 1
    var state: u32 = *seed * 747796405u + 2891336453u;

    var word: u32 = (state >> ((state >> 28u) + 4u)) ^ state;
    word = word * 277803737u;

    // change seed value in place
    *seed = (word >> 22u) ^ word;

    return normalize_u32(*seed);
}


fn random_normal_scaler(seed: ptr<function, u32>) -> vec3<f32>{
    // normally distributed random vec3 scaler from -1 to 1
    var scaler = vec3<f32>(0.0);
    scaler.x = normal_distribution(seed);
    scaler.y = normal_distribution(seed);
    scaler.z = normal_distribution(seed);

    return scaler;
}

fn random_scaler(seed: ptr<function, u32>) -> vec3<f32>{
    // random vec3 scaler from -1 to 1
    var scaler = vec3<f32>(0.0);
    scaler.x = pcg_hash(seed);
    scaler.y = pcg_hash(seed);
    scaler.z = pcg_hash(seed);

    return scaler * 2.0 - 1.0;
}

fn normal_distribution(seed: ptr<function, u32>) -> f32{
    // returns normally distributed float
    let theta: f32 = 2.0 * 3.1415926 * pcg_hash(seed);
    let rho: f32 = sqrt(-2.0 * log(pcg_hash(seed)));
    return rho * cos(theta);

}

fn normalize_u32(value: u32) -> f32{
    return f32(value) / f32(U32_MAX);
}


