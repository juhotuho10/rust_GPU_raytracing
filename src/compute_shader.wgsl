const F32_MAX: f32 = 3.4028235e+38;
const U32_MAX: u32 = 4294967295u;
const PI: f32 = 3.1415926536;


@group(0) @binding(0) var<storage, read> params: Params;
@group(0) @binding(1) var<storage, read> camera_rays: array<vec3<f32>>;
@group(0) @binding(2) var<storage, read_write> output_data: array<u32>;
@group(0) @binding(3) var<uniform> ray_camera: RayCamera;
@group(0) @binding(4) var<uniform> material_array: array<SceneMaterial, 19>;
@group(0) @binding(5) var<uniform> sphere_array: array<SceneSphere, 3>;
@group(0) @binding(6) var<storage, read_write> accumulation_data: array<vec4<f32>>;
@group(0) @binding(7) var<storage, read> triangle_array: array<SceneTriangle, 5552>;
@group(0) @binding(8) var<uniform> object_array: array<ObjectInfo, 34>;
@group(0) @binding(9) var texture_array: texture_2d_array<f32>;
@group(0) @binding(10) var<storage, read> sub_object_array: array<SubObjectInfo, 802>;
@group(0) @binding(11) var environment_map: texture_2d<f32>;


fn sample_texture(index: u32, coords: vec2<f32>, texture_size: vec2<i32>) -> vec4<f32> {
    let texel_coords = vec2<i32>(coords * vec2<f32>(texture_size));

    let color = textureLoad(texture_array, texel_coords, i32(index), 0);

    return color;
}

fn sample_env_map(coords: vec2<f32>, texture_size: vec2<i32>) -> vec4<f32> {
    let texel_coords = vec2<i32>(coords * vec2<f32>(texture_size));

    let color = textureLoad(environment_map, texel_coords, 0);

    return color;
}


struct Params {
    width: u32,
    accumulation_index: u32,
    accumulate: u32,
    sphere_count: u32,   
    object_count: u32, 
    compute_per_frame: u32,
    texture_width: u32,
    texture_height: u32,
    textue_count: u32,
    env_map_width: u32,
    env_map_height: u32,
    // explicit padding to match 16 byte alignment
    _padding1: u32,
      
};


struct RayCamera {
    origin: vec3<f32>,    

    // explicit padding to match 16 byte alignment
     _padding1: u32,
};

struct SceneMaterial {
    texture_index: u32,         
    roughness: f32,
    emission_power: f32,            
    specular: f32,            
    specular_scatter: f32,
    glass: f32,
    refraction_index: f32,
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
    _padding3: u32,       
}

struct SceneTriangle {
    a: vec3<f32>,
    _padding: u32,
    edge_ab: vec3<f32>,
    _padding2: u32,
    edge_ac: vec3<f32>,
    _padding3: u32,
    calc_normal: vec3<f32>,
    _padding4: u32,
    face_normal: vec3<f32>,
    _padding5: u32,
    min_bounds: vec3<f32>,
    _padding6: u32,
    max_bounds: vec3<f32>,
    _padding7: u32,
    // explicit padding to match 16 byte alignment 
}

struct ObjectInfo {
    min_bounds: vec3<f32>,
    first_sub_object_index: u32,
    max_bounds: vec3<f32>,
    sub_object_count: u32,
    material_index: u32,
    _padding1: u32,
    _padding2: u32,
    _padding3: u32,
}

struct SubObjectInfo {
    min_bounds: vec3<f32>,
    first_triangle_index: u32,
    max_bounds: vec3<f32>,
    triangle_count: u32,
}



struct HitPayload {
    hit_distance: f32,
    world_position: vec3<f32>,
    hitside_normal: vec3<f32>,
    material_index: u32,
    front_face: bool,
    texture_point: vec2<f32>
}

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
}



@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index: u32 =  (global_id.y * params.width) + global_id.x;

    let bounces: u32 = 10u;

    var render_color = vec4<f32>(0.0);

    var random_index = params.accumulation_index;

    var pixel_color: vec4<f32> = accumulation_data[index];

    if params.accumulate == 1{

        for (var i: u32 = 0u; i < params.compute_per_frame; i = i + 1) {
            pixel_color += per_pixel(index, bounces, random_index);
            random_index = random_index + 1;
        }
        accumulation_data[index] = pixel_color;

        var accumulated_color: vec4<f32> = pixel_color / f32(params.accumulation_index * params.compute_per_frame);

        render_color = clamp(accumulated_color, vec4<f32>(0.0), vec4<f32>(1.0));
        

    }else{

        let f32_color: vec4<f32> = per_pixel(index, bounces, random_index);
        render_color = clamp(f32_color, vec4<f32>(0.0), vec4<f32>(1.0));
    }
    
    // pack 4 f32 values into a single u32 (4x u8 rgba color)
    output_data[index] = pack_to_u32(render_color);


    /*let count: u32 = object_array[0].triangle_count;

    if (count == 202) {
        output_data[index] = pack_to_u32(vec3<f32>(0.0, 1.0, 0.0)); // green
    } else {
        output_data[index] = pack_to_u32(vec3<f32>(1.0, 0.0, 0.0)); // red
    };*/

}


fn pack_to_u32(vector: vec4<f32>) -> u32 {
  // scale the f32 values from [0.0, 1.0] to [0.0, 255.0]
    let scaled_r: u32 = u32(vector.r * 255.0);
    let scaled_g: u32 = u32(vector.g * 255.0);
    let scaled_b: u32 = u32(vector.b * 255.0);
    let scaled_a: u32 = u32(vector.a * 255.0);


    // extract the least significant 8 bits (same as converting to u8)
    let byte0: u32 = scaled_r & 0xFFu;
    let byte1: u32 = scaled_g & 0xFFu;
    let byte2: u32 = scaled_b & 0xFFu;
    let byte3: u32 = scaled_a & 0xFFu;

    // pack the bits into a single u32 that will then be read as 4x u8 by the rendering pass
    return (byte0 << 0) | (byte1 << 8) | (byte2 << 16) | (byte3 << 24);
}

fn per_pixel(index: u32, bounces: u32, random_index: u32) -> vec4<f32> {

    var ray = Ray( 
        ray_camera.origin,
        camera_rays[index]
    );

    var seed: u32 = index * random_index * 326624u;

    ray.direction += random_scaler(&seed) * 0.0005;
    
    var light_contribution = vec4<f32>(1.0);
    var light = vec4<f32>(0.0);

    

    for (var i: u32 = 0u; i < bounces; i = i + 1) {

        let hit_payload: HitPayload = trace_ray(ray);

        if hit_payload.hit_distance == F32_MAX {
            
            // we hit the sky

            let uv: vec2<f32> = environment_map_coords(ray.direction);
            let texture_size = vec2<i32>(i32(params.env_map_width), i32(params.env_map_height));

            let color: vec4<f32>  = sample_env_map(uv, texture_size);

            light += color * light_contribution;
            break;
        }

        let material_index: u32 = hit_payload.material_index;
        let current_material: SceneMaterial = material_array[material_index];

        let diffuse_direction: vec3<f32> = normalize(hit_payload.hitside_normal + random_normal_scaler(&seed));
        let specular_direction: vec3<f32> = reflect(ray.direction, hit_payload.hitside_normal);
   
        let texture_size = vec2<i32>(i32(params.texture_width), i32(params.texture_height));

        let current_color: vec4<f32> = sample_texture(current_material.texture_index, hit_payload.texture_point, texture_size);

        let emitted_light = current_color * current_material.emission_power;
        light += emitted_light * light_contribution;

        let is_glass: bool = current_material.glass > random(&seed);

        if is_glass{

            var refraction_index: f32 = current_material.refraction_index;

            if hit_payload.front_face{
                refraction_index = 1.0 /refraction_index;
            }

            let cos_theta: f32 = min(dot(-ray.direction, hit_payload.hitside_normal), 1.0);

            let sin_theta: f32 = sqrt(1.0 - cos_theta * cos_theta);

            let reflects: bool = refraction_index * sin_theta > 1.0;

            let specular_percentage: f32 = specular_percentage(cos_theta, refraction_index);

            let is_specular: bool = (current_material.specular * specular_percentage) > random(&seed);

            if reflects || is_specular {
                // specular reflection, bounces off the glass
                
                ray.direction = lerp(specular_direction, diffuse_direction, current_material.specular_scatter);
                ray.origin = hit_payload.world_position + hit_payload.hitside_normal * 0.0001;

            } else { 
                // refraction, goes through the glass

                let refraction_direction: vec3<f32> = refract(ray.direction, hit_payload.hitside_normal, cos_theta, refraction_index);
                
                // normal roughness calculation in wayy to harsh for glass, 1/10 is plenty
                ray.direction = lerp(refraction_direction, diffuse_direction, current_material.roughness / 10.0);

                // ray goes through the material so we want it to be set on the opposite side of the hitside normal
                ray.origin = hit_payload.world_position - hit_payload.hitside_normal * 0.0001;

                light_contribution *= current_color; 
            }

        }else{

            let is_specular_bounce: bool = current_material.specular > random(&seed);

            if is_specular_bounce{
                ray.direction = lerp(specular_direction, diffuse_direction, current_material.specular_scatter);

            }else{
                ray.direction = lerp(specular_direction, diffuse_direction, current_material.roughness);
                light_contribution *= current_color;
            }

            ray.origin = hit_payload.world_position + hit_payload.hitside_normal * 0.0001;

        }

    }
    return light;
}

fn refract(ray_direction:vec3<f32>, hitside_normal: vec3<f32>, cos_theta: f32, refraction_index: f32) -> vec3<f32>{
    // the ray goes through the glass and the direction is changed based on refraction index

    let ray_perpendicular: vec3<f32> =  refraction_index * (ray_direction + cos_theta * hitside_normal);

    let len_squared = length(ray_perpendicular) * length(ray_perpendicular);
    let ray_parallel: vec3<f32> = -sqrt(abs(1.0 - len_squared)) * hitside_normal;

    return ray_perpendicular + ray_parallel;
}


fn specular_percentage(cos_theta: f32, refraction_index: f32) -> f32{
    // Schlick's approximation for reflectance
    var refraction_0 = (1 - refraction_index) / (1 + refraction_index);
    refraction_0 = refraction_0 * refraction_0;
    return refraction_0 + (1-refraction_0) * pow((1.0 - cos_theta), 5.0);

}

fn lerp(start: vec3<f32>, end: vec3<f32>, t: f32) -> vec3<f32>{
    // smooth transition between 2 vectors from 0.0 to 1.0
    return start + (end - start) * t;
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
    for (var sphere_index: i32 = 0; sphere_index < i32(params.sphere_count); sphere_index = sphere_index + 1) {
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


fn ray_in_bounds(ray: Ray, min_bounds: vec3<f32>, max_bounds: vec3<f32>) -> bool{

    // quick check to see if the ray falls within the object bounds

    let inv_direction: vec3<f32> = 1 / ray.direction;
    let min_t: vec3<f32> = (min_bounds - ray.origin) * inv_direction;
    let max_t: vec3<f32> = (max_bounds - ray.origin) * inv_direction;
    let t1: vec3<f32> = min(min_t, max_t);
    let t2: vec3<f32> = max(min_t, max_t);
    let near_t: f32 = max(max(t1.x, t1.y), t1.z);
    let far_t: f32 = min(min(t2.x, t2.y), t2.z);
    return near_t <= far_t && far_t >= 0.0;
}


fn check_triangles(ray: Ray) -> HitPayload{

    var closest_distance = F32_MAX;
    var closest_hitpayload: HitPayload = miss();

    for (var object_index: u32 = 0; object_index < params.object_count; object_index = object_index + 1) {
        let object_info: ObjectInfo = object_array[object_index];

        // quick way to filter out objects that can't be hit with ray
        if !ray_in_bounds(ray, object_info.min_bounds, object_info.max_bounds){
            continue;
        }

        for (var i: u32 = 0; i < object_info.sub_object_count; i = i + 1){
            let sub_object_index = object_info.first_sub_object_index + i;

            let sub_object_info: SubObjectInfo = sub_object_array[sub_object_index];

                        
            if !ray_in_bounds(ray, sub_object_info.min_bounds, sub_object_info.max_bounds){
                continue;
            }

            for (var j: u32 = 0; j < sub_object_info.triangle_count; j = j + 1) {
                let triangle_index = sub_object_info.first_triangle_index + j;
                let tri: SceneTriangle = triangle_array[triangle_index];
                
                let determinant: f32 = -dot(ray.direction, tri.calc_normal);

                let inv_det: f32 = 1 / determinant;
                
                let ao: vec3<f32> = ray.origin - tri.a; 

                let distance: f32 = dot(ao, tri.calc_normal) * inv_det;

                if distance < 0.0 || distance >= closest_distance {
                    continue;
                }

                let dao: vec3<f32> = cross(ao, ray.direction); 

                // calculate distance and intersection

                let v: f32 = -dot(tri.edge_ab, dao) * inv_det;

                if v < 0.0 {
                    continue;
                }
                
                let u: f32 = dot(tri.edge_ac, dao) * inv_det;

                if u < 0.0 {
                    continue;
                }
                
                let w: f32 = 1 - u - v;

                if w < 0.0 {
                    continue;
                }

                var front_face: bool;

                var hitside_normal: vec3<f32>;

                if determinant > 0.0 {
                    front_face = true;
                    hitside_normal = tri.face_normal;
                }else{
                    front_face = false;
                    hitside_normal = -tri.face_normal;
                }

                closest_distance = distance;

                let hitpoint = ray.origin + ray.direction * distance;

                let texture_coords = object_texture_coords(hitpoint, object_info.min_bounds, object_info.max_bounds);


                closest_hitpayload = HitPayload(
                    distance,
                    hitpoint,
                    hitside_normal,
                    object_info.material_index,
                    front_face,
                    texture_coords,
                );
    
            };
        }
    };

    return closest_hitpayload;

}

fn miss() -> HitPayload{ 
    return HitPayload(F32_MAX, 
    vec3<f32>(0.0),
    vec3<f32>(0.0),
    0u,
    false,
    vec2<f32>(0.0)
    );
}


fn sphere_hit(ray: Ray, hit_distance: f32, object_index: u32) -> HitPayload{
    let closest_sphere: SceneSphere = sphere_array[object_index];

    let hit_point: vec3<f32> = ray.origin + ray.direction * hit_distance;
    var outward_normal: vec3<f32> = normalize(hit_point - closest_sphere.position);

    let texture_coords: vec2<f32> = sphere_texture_coords(outward_normal);

    let front_face = dot(ray.direction, outward_normal) < 0;

    var hitside_normal: vec3<f32>;

    if front_face{
        hitside_normal = outward_normal;
    }else{
        hitside_normal = -outward_normal;
    }

    return HitPayload(hit_distance, 
    hit_point,
    hitside_normal,
    closest_sphere.material_index,
    front_face,
    texture_coords,
    );
}

fn sphere_texture_coords(outward_normal: vec3<f32>) -> vec2<f32>{

    let theta: f32 = acos(-outward_normal.y);
    let phi: f32 = atan2(-outward_normal.z, outward_normal.x) + PI;

    let u: f32 = phi / (2.0 * PI);
    let v: f32 = theta / PI;

    return vec2<f32>(u, v);
}

fn object_texture_coords(hitpoint: vec3<f32>, min_bounds: vec3<f32>, max_bounds: vec3<f32>) -> vec2<f32>{

    let bounds_range = max_bounds - min_bounds;

    let normalized_hitpoint = (hitpoint - min_bounds) / bounds_range;

    let u = normalized_hitpoint.x;
    let v = normalized_hitpoint.z;

    return vec2<f32>(u, v);
}

fn environment_map_coords(ray_direction: vec3<f32>) -> vec2<f32>{
    let u = 0.5 + atan2(ray_direction.z, ray_direction.x) / (2.0 * PI);
    let v = 0.5 + asin(ray_direction.y) / PI;

    return vec2<f32>(u, v);
}

fn random(seed: ptr<function, u32>) -> f32 {

    // random float between 0 and 1 using pcg hash
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
    scaler.x = random(seed);
    scaler.y = random(seed);
    scaler.z = random(seed);

    return scaler * 2.0 - 1.0;
}

fn normal_distribution(seed: ptr<function, u32>) -> f32{
    // returns normally distributed float
    let theta: f32 = 2.0 * 3.1415926 * random(seed);
    let rho: f32 = sqrt(-2.0 * log(random(seed)));
    return rho * cos(theta);

}

fn normalize_u32(value: u32) -> f32{
    return f32(value) / f32(U32_MAX);
}


