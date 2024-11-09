struct Camera {
    view_proj: mat4x4<f32>,
}

struct DebugVertex {
    @location(0)
    position: vec3<f32>,
    @location(1)
    color: vec3<f32>,
}

@group(0)
@binding(0)
var<uniform> camera: Camera;

struct VsOut {
    @location(0)
    color: vec3<f32>,
    @builtin(position)
    frag_position: vec4<f32>,
}

@vertex
fn displace_vertices(vertex: DebugVertex) -> VsOut {
    let frag_position = camera.view_proj * vec4(vertex.position, 1.0);
    return VsOut(vertex.color, frag_position);
}

@fragment
fn draw(vs: VsOut) -> @location(0) vec4<f32> {
    return vec4(vs.color, 1.0);
}