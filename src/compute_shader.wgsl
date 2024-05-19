struct Params {
    width: u32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> input_data: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> output_data: array<u32>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index =  (global_id.y * params.width) + global_id.x;

    let pixel = input_data[index];

    if (pixel.x == pixel.y) {
        output_data[index] = pack_to_u32(0.0, 1.0, 0.0); // green
    } else {
        output_data[index] = pack_to_u32(1.0, 0.0, 0.0); // red
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