#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_clip}
#import bevy_pbr::mesh_view_bindings::view

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,

    @location(3) i_pos_scale: vec4<f32>,
    @location(4) i_color: vec4<f32>,
    @location(5) i_normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) world_position: vec3<f32>,
    @location(2) color: vec4<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    let position = vertex.position * vertex.i_pos_scale.w + vertex.i_pos_scale.xyz;
    let world_position = get_world_from_local(0u) * vec4<f32>(position, 1.0);

    var out: VertexOutput;
    out.clip_position = mesh_position_local_to_clip(get_world_from_local(0u), vec4<f32>(position, 1.0));
    out.world_position = world_position.xyz;
    out.world_normal = normalize((get_world_from_local(0u) * vec4<f32>(vertex.normal + vertex.i_normal, 0.0)).xyz);
    out.color = vertex.i_color;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let diffuse = max(dot(in.world_normal, light_dir), 0.0);
    let ambient = 0.1;
    let final_color = in.color.rgb * (diffuse + ambient);
    return vec4<f32>(final_color, in.color.a);
}