struct Camera {
    view_proj: mat4x4<f32>,
}

struct Vertex {
    @location(0)
    position: vec3<f32>,
    @location(1)
    normal: vec3<f32>,
    @location(2)
    tex_coord: vec2<f32>,
}

struct Instance {
    @builtin(instance_index)
    id: u32,
}

@group(0)
@binding(0)
var<uniform> camera: Camera;

struct VsOut {
    @location(0)
    world_normal: vec3<f32>,
    @location(1)
    tex_coord: vec2<f32>,
    @builtin(position)
    frag_position: vec4<f32>,
}

@vertex
fn displace_vertices(vertex: Vertex, shell: Instance) -> VsOut {
    let normal = vertex.normal;
    let num_shells = 32.0;
    let factor = f32(shell.id) / num_shells * 0.5;
    let displaced = vertex.position + vertex.normal * factor;
    let frag_position = camera.view_proj * vec4(displaced, 1.0);
    return VsOut(normal, vertex.tex_coord, frag_position);
}

@fragment
fn shade_fur(in: VsOut) -> @location(0) vec4<f32> {
    let color = in.world_normal * 0.5 + 0.5;
    return vec4(color, 1.0);
}