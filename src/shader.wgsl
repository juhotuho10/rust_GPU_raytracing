//struct VertexOutput {
//    @builtin(position) position: vec4<f32>,
//    @location(1) tex_coords: vec2<f32>,
//};
//
//@group(0) @binding(0) var tex: texture_2d<f32>;
//@group(0) @binding(1) var samp: sampler;


//@vertex
//fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
//    //positions to cover the whole NDC space [-1, 1]
//    var positions = array<vec4<f32>, 6>(
//        vec4<f32>(-1.0,  1.0, 0.0, 1.0), // top left
//        vec4<f32>(-1.0, -1.0, 0.0, 1.0), // bottom left
//        vec4<f32>( 1.0, -1.0, 0.0, 1.0), // bottom right
//        vec4<f32>(-1.0,  1.0, 0.0, 1.0), // top left
//        vec4<f32>( 1.0, -1.0, 0.0, 1.0), // bottom right
//        vec4<f32>( 1.0,  1.0, 0.0, 1.0)  // top right
//    );
//
//    //texture coordinates corresponding to each vertex
//    var tex_coords = array<vec2<f32>, 6>(
//        vec2<f32>(0.0, 0.0), // top left
//        vec2<f32>(0.0, 1.0), // bottom left
//        vec2<f32>(1.0, 1.0), // bottom right
//        
//        vec2<f32>(0.0, 0.0), // top left
//        vec2<f32>(1.0, 1.0), // bottom right
//        vec2<f32>(1.0, 0.0)  // top right
//    );
//
//    return VertexOutput(positions[vertex_index], tex_coords[vertex_index]);
//}
//
//
//@fragment
//fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
//    return textureSample(tex, samp, input.tex_coords);
//}


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

fn pack_4u32_to_u32(x: u32, y: u32, z: u32, w: u32) -> u32 {
    let byte0: u32 = x & 0xFFu;
    let byte1: u32 = y & 0xFFu;
    let byte2: u32 = z & 0xFFu;
    let byte3: u32 = w & 0xFFu;

    return (byte0 << 0) | (byte1 << 8) | (byte2 << 16) | (byte3 << 24);
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(1) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    //positions to cover the whole NDC space [-1, 1]
    var positions = array<vec4<f32>, 6>(
        vec4<f32>(-1.0,  1.0, 0.0, 1.0), // top left
        vec4<f32>(-1.0, -1.0, 0.0, 1.0), // bottom left
        vec4<f32>( 1.0, -1.0, 0.0, 1.0), // bottom right
        vec4<f32>(-1.0,  1.0, 0.0, 1.0), // top left
        vec4<f32>( 1.0, -1.0, 0.0, 1.0), // bottom right
        vec4<f32>( 1.0,  1.0, 0.0, 1.0)  // top right
    );
//
    //texture coordinates corresponding to each vertex
    var tex_coords = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), // top left
        vec2<f32>(0.0, 1.0), // bottom left
        vec2<f32>(1.0, 1.0), // bottom right
        
        vec2<f32>(0.0, 0.0), // top left
        vec2<f32>(1.0, 1.0), // bottom right
        vec2<f32>(1.0, 0.0)  // top right
    );
//
    return VertexOutput(positions[vertex_index], tex_coords[vertex_index]);
}

@group(0) @binding(3) var my_texture: texture_2d<f32>;
@group(0) @binding(4) var my_sampler: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.tex_coords.x, input.tex_coords.y, 0.0, 1.0);
    //return textureSample(my_texture, my_sampler, input.tex_coords);
    
}
