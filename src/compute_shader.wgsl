struct Params {
    width: u32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> input_data: array<vec3<f32>>;
@group(0) @binding(2) var<storage, read_write> output_data: array<u32>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index =  global_id.y * params.width - global_id.x;
    let pixel = input_data[index];

    if (pixel.x == pixel.y) {
        output_data[index] = pack_4u32_to_u32(0u, 255u,0u,255u); //vec4<f32>(1.5, 1.5, 1.5, 1.5); // Green
    } else {
        output_data[index] = pack_4u32_to_u32(255u, 0u,0u,255u); //vec4<f32>(0, 0, 0, 0); // Red
    }
}

fn pack_4u32_to_u32(x: u32, y: u32, z: u32, w: u32) -> u32 {
    let byte0: u32 = x & 0xFFu;
    let byte1: u32 = y & 0xFFu;
    let byte2: u32 = z & 0xFFu;
    let byte3: u32 = w & 0xFFu;

    return (byte0 << 0) | (byte1 << 8) | (byte2 << 16) | (byte3 << 24);
}