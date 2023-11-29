struct VertexOutput {
    @builtin(position)
    position: vec4<f32>,
    @location(0)
    normals: vec4<f32>,
};

var<push_constant> projview: mat4x4<f32>;

@group(0)
@binding(0)
var<uniform> transformations: array<mat4x4<f32>, 64>;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normals: vec3<f32>,
    // apparently it's not valid to use a vec3<i32> in a vertex buffer
    // so just coercing floats back into integers
    @location(2) joints: vec3<f32>,
    @location(3) weights: vec3<f32>,
) -> VertexOutput {
    let joints = vec3(i32(joints.x), i32(joints.y), i32(joints.z));

    var total_pos: vec4<f32> = vec4<f32>(0.0);
    var total_norms: vec4<f32> = vec4<f32>(0.0);

    var i: u32 = 0u;
    loop {
        if (i >= 3u) {
            break;
        }
        let local_pos = transformations[joints[i]] * vec4<f32>(position, 1.0);
        total_pos = total_pos + local_pos * weights[i];
        let local_norms = transformations[joints[i]] * vec4<f32>(normals, 0.0);
        total_norms = total_norms + local_norms * weights[i];
        continuing {
            i = i + 1u;
        }
    }

    var result: VertexOutput;
    result.position = projview * total_pos;
    result.normals = total_norms;
    return result;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    // return vec4<f32>(1.0, 0.0, 1.0, 1.0);
    return vertex.normals;
}
