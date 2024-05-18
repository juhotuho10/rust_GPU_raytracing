

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(1) tex_coords: vec2<f32>,
};

@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;


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

    //texture coordinates corresponding to each vertex
    var tex_coords = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), // top left
        vec2<f32>(0.0, 1.0), // bottom left
        vec2<f32>(1.0, 1.0), // bottom right
        
        vec2<f32>(0.0, 0.0), // top left
        vec2<f32>(1.0, 1.0), // bottom right
        vec2<f32>(1.0, 0.0)  // top right
    );

    return VertexOutput(positions[vertex_index], tex_coords[vertex_index]);
}


@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(tex, samp, input.tex_coords);
}