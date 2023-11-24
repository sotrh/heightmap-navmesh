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
    @location(2)
    height_factor: f32,
    @location(3)
    debug: vec3<f32>,
    @builtin(position)
    frag_position: vec4<f32>,
}

@vertex
fn displace_vertices(vertex: Vertex, shell: Instance) -> VsOut {
    let normal = vertex.normal;
    let num_shells = 32.0;
    let fur_height = 0.1;
    let height_factor = f32(shell.id) / num_shells;
    let displaced = vertex.position + vertex.normal * height_factor * fur_height;
    let frag_position = camera.view_proj * vec4(displaced, 1.0);
    return VsOut(normal, vertex.tex_coord, height_factor, vec3(displaced), frag_position);
}

@fragment
fn shade_fur(in: VsOut) -> @location(0) vec4<f32> {
    // let color = in.world_normal * 0.5 + 0.5;
    // let color = vec3(in.tex_coord, 0.0);

    let p = in.tex_coord * 100.0;
    let grid_cell = floor(p);
    let noise = rand(grid_cell);
    if noise < in.height_factor {
        discard;
    }

    let g = fract(p);
    let d = distance(vec2(0.5), g);

    let color = vec3(1.0 - d);

    return vec4(color * in.height_factor, 1.0);
}

fn rand(co: vec2<f32>) -> f32 {
    return fract(sin(dot(co, vec2(12.9898, 78.233))) * 43758.5453);
}

// float rand(vec2 co){
//     return fract(sin(dot(co, vec2(12.9898, 78.233))) * 43758.5453);
// }

fn permute3(x: vec3<f32>) -> vec3<f32> { return (((x * 34.) + 1.) * x) % vec3<f32>(289.); }

fn snoise2(v: vec2<f32>) -> f32 {
    let C = vec4<f32>(0.211324865405187, 0.366025403784439, -0.577350269189626, 0.024390243902439);
    var i: vec2<f32> = floor(v + dot(v, C.yy));
    let x0 = v - i + dot(i, C.xx);
    // I flipped the condition here from > to < as it fixed some artifacting I was observing
    var i1: vec2<f32> = select(vec2<f32>(1., 0.), vec2<f32>(0., 1.), (x0.x < x0.y));
    var x12: vec4<f32> = x0.xyxy + C.xxzz - vec4<f32>(i1, 0., 0.);
    i = i % vec2<f32>(289.);
    let p = permute3(permute3(i.y + vec3<f32>(0., i1.y, 1.)) + i.x + vec3<f32>(0., i1.x, 1.));
    var m: vec3<f32> = max(0.5 - vec3<f32>(dot(x0, x0), dot(x12.xy, x12.xy), dot(x12.zw, x12.zw)), vec3<f32>(0.));
    m = m * m;
    m = m * m;
    let x = 2. * fract(p * C.www) - 1.;
    let h = abs(x) - 0.5;
    let ox = floor(x + 0.5);
    let a0 = x - ox;
    m = m * (1.79284291400159 - 0.85373472095314 * (a0 * a0 + h * h));
    let g = vec3<f32>(a0.x * x0.x + h.x * x0.y, a0.yz * x12.xz + h.yz * x12.yw);
    return 130. * dot(m, g);
}