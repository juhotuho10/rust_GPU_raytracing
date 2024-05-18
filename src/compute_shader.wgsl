struct Params {
    width: u32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> input_data: array<vec3<f32>>;
@group(0) @binding(2) var<storage, read_write> output_data: array<vec4<f32>>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x + global_id.y * params.width;
    let pixel = input_data[index];

    if (pixel.x == pixel.y) {
        output_data[index] = vec4<f32>(0.0, 255.0, 0.0, 255.0); // Green
    } else {
        output_data[index] = vec4<f32>(255.0, 0.0, 0.0, 255.0); // Red
    }
}