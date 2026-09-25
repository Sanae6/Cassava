@group(0) @binding(0) var<uniform> proj_view: mat4x4f;
@group(1) @binding(0) var<uniform> model_mtx: mat4x4f;

@if(textured) @group(1) @binding(1) var t_diffuse: texture_2d<f32>;
@if(textured) @group(1) @binding(2) var s_diffuse: sampler;

struct VertexInput {
    @location(0) position: vec3f,
    @if(textured) @location(1) uv: vec2f,
    @if(vertex_colours) @location(2) colour: vec4f,
    @location(3) normal: vec3f,
}
struct VertexOutput {
    @builtin(position) position: vec4f,
    @if(textured) @location(0) uv: vec2f,
    @if(vertex_colours) @location(1) colour: vec4f,
    @location(2) normal: vec3f,
}
@vertex
fn vtx_main(
    in: VertexInput
) -> VertexOutput {
    var out: VertexOutput;

    out.position = proj_view * model_mtx * vec4f(in.position, 1);
    @if(textured) out.uv = in.uv;
    @if(vertex_colours) out.colour = in.colour;
    out.normal = in.normal;
    return out;
}

@fragment
fn frag_main(in: VertexOutput) -> @location(0) vec4f {
    @if(textured && vertex_colours) return in.colour * textureSample(t_diffuse, s_diffuse, in.uv);
    @elif(textured) return textureSample(t_diffuse, s_diffuse, in.uv);
    @elif(vertex_colours) return in.colour;
    @else return vec4f(in.normal, 1);
}
