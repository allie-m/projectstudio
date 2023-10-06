// terrain pipeline (one bind, one draw call)
// - depth only, no fragment
// - full color (w/shadow map)
// decoration pipeline (rebinds/multiple calls for texture/model combos; instancing)
// - depth only, no fragment
// - full color (w/shadow map)
// (raymarched?) cloud shader (one draw call; see Sebastian Lague's implementation)
// water pipeline (references reflection/refraction textures)
// postprocessing pipeline (if applicable)
//
// shadow/water maps are generated with depth only terrain+decoration+cloud/sky
// perspective render is generated w/full color terrain+decoration+water pipelines
// postprocessing is then applied (if applicable)

// TODO: move these
// https://stackoverflow.com/questions/40539825/normals-of-height-map-dont-work
// https://www.youtube.com/watch?v=UXD97l7ZT0w&list=PLFt_AvWsXl0dT82XMtKATYPcVIhpu2fh6&index=2
// https://www.cs.cmu.edu/~garland/scape/scape.pdf
// https://web.eecs.umich.edu/~sugih/courses/eecs494/fall06/lectures/workshop-terrain.pdf
// https://web.archive.org/web/20221129211147/https://scottin3d.com/blog/mesh-generation-from-heightmaps
// https://github.com/heremaps/tin-terrain
// https://github.com/heremaps/tin-terrain/blob/master/docs/Terra.md

struct PushConstants {
    projview: mat4x4<f32>,
    chunk_size: u32,
    normal_index: i32,
}

var<push_constant> push_constants: PushConstants;

@vertex
fn terrain_vertex_bake(
    @builtin(vertex_index) vtx: u32,
    @location(0) position: vec3<f32>,
    @location(1) norms: vec3<f32>,
) -> @builtin(position) vec4<f32> {
    return push_constants.projview * vec4(position, 1.0);
}

struct TerrainVertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) norms: vec3<f32>,
}

@vertex
fn terrain_vertex(
    @builtin(vertex_index) vtx: u32,
    @location(0) position: vec3<f32>,
    @location(1) norms: vec3<f32>,
) -> TerrainVertexOutput {
    var out: TerrainVertexOutput;
    out.pos = push_constants.projview * vec4(position, 1.0);
    out.norms = norms;
    return out;
}

@group(0) @binding(0) var normals: texture_2d_array<f32>;
@group(0) @binding(1) var sam: sampler;

@fragment
fn terrain_fragment(v_in: TerrainVertexOutput) -> @location(0) vec4<f32> {
    // let l_x = fract(v_in.loc.x / f32(push_constants.chunk_size));
    // let l_y = fract(v_in.loc.y / f32(push_constants.chunk_size));
    // let normal = textureSampleLevel(normals, sam, vec2(l_x, l_y), push_constants.normal_index, 0.0);

    let normal = v_in.norms;

    // test lighting
    let test = vec4(max(dot(normal.xyz, -normalize(vec3(0.0, -1.0, -5.0))), 0.05) * vec3(0.2, 0.8, 0.1), 1.0);

    return test;
}

// @vertex
// fn water_vertex(
//     @location(0) position: vec3<f32>,
// ) -> @builtin(position) vec4<f32> {
//     return push_constants.projview * vec4(position, 1.0);
// }

// @fragment
// fn water_fragment() -> @location(0) vec4<f32> {
//     return vec4(0.0, 0.0, 1.0, 1.0);
// }
