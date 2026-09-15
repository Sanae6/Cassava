@binding(0) @group(0) var<uniform> proj_view: mat4x4f;
@binding(0) @group(1) var<uniform> model_mtx: mat4x4f;

struct VertexInput {
    @location(0) position: vec3f,
    // @location(1) uv: vec2f,
    // @location(2) colour: vec4f,
    @location(3) normal: vec3f,
}
struct VertexOutput {
    @builtin(position) position: vec4f,
    // @location(0) uv: vec2f,
    // @location(1) colour: vec4f,
    @location(2) normal: vec3f,
}
@vertex
fn vtx_main(
    in: VertexInput
) -> VertexOutput {
    var out: VertexOutput;

    out.position = proj_view * model_mtx * vec4f(in.position, 1);
    // out.uv = in.uv;
    // out.colour = in.colour;
    out.normal = in.normal;
    return out;
}

@fragment
fn frag_main(in: VertexOutput) -> @location(0) vec4f {
    return vec4f(in.normal, 1);
}
