@binding(0) @group(0) var<uniform> proj_view: mat4x4<f32>;

struct Instance {
    @location(0) mtx_0: vec4<f32>,
    @location(1) mtx_1: vec4<f32>,
    @location(2) mtx_2: vec4<f32>,
    @location(3) mtx_3: vec4<f32>,
}
@vertex
fn vtx_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: Instance
) -> @builtin(position) vec4f {
    const pos = array(
        vec2(0.0, 0.5),
        vec2(-0.5, -0.5),
        vec2(0.5, -0.5)
    );

    var in_mtx = mat4x4<f32>(
        instance.mtx_0,
        instance.mtx_1,
        instance.mtx_2,
        instance.mtx_3,
    );

    return proj_view * in_mtx * vec4f(pos[vertex_index], 0, 1);
}

@fragment
fn frag_main() -> @location(0) vec4f {
    return vec4(0, 0, 1, 1);
}
