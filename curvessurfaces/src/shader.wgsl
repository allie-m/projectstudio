struct VertexOutput {
    @builtin(position)
    position: vec4<f32>,
    @location(0)
    normals: vec3<f32>,
};

var<push_constant> projview: mat4x4<f32>;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normals: vec3<f32>,
) -> VertexOutput {
    var result: VertexOutput;
    result.position = projview * vec4<f32>(position, 1.0);
    result.normals = normals;
    return result;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    // return vec4<f32>(1.0, 0.0, 1.0, 1.0);
    return vec4<f32>(vertex.normals, 1.0);
}
