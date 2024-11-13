struct Particle {
    pos: vec3<f32>,
    // implicit padding (4 bytes)
    vel: vec3<f32>,
    // implicit padding (4 bytes)
    norm: vec3<f32>,
    // implicit padding (4 bytes)
}

// must be a power of 2
let cloth_size: u32 = 128u;

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;

fn set_normal(gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    var normal: vec3<f32>;

    let pa = particles[x + y * cloth_size];

    // get the plane of the 4 particles adjacent this one (3 to make a plane, so gotta average the composite triangles)
    // if we are on the edge we extrapolate from the point on the same axis in the other direction
    // so, if no x + 1, we do 2*p[x,y] - p[x-1,y]
    // get the vector perpendicular to this plane (normalized & at 0)
    
    var a: vec3<f32>;
    var b: vec3<f32>;
    var c: vec3<f32>;
    var d: vec3<f32>;
    if (x != 0u) {
        a = particles[(x - 1u) + y * cloth_size].pos;
    } else {
        a = 2.0 * pa.pos - particles[(x + 1u) + y * cloth_size].pos;
    }
    if (y != 0u) {
        b = particles[x + (y - 1u) * cloth_size].pos;
    } else {
        b = 2.0 * pa.pos - particles[x + (y + 1u) * cloth_size].pos;
    }
    if (x != (cloth_size - 1u)) {
        c = particles[(x + 1u) + y * cloth_size].pos;
    } else {
        c = 2.0 * pa.pos - particles[(x - 1u) + y * cloth_size].pos;
    }
    if (y != (cloth_size - 1u)) {
        d = particles[x + (y + 1u) * cloth_size].pos;
    } else {
        d = 2.0 * pa.pos - particles[x + (y - 1u) * cloth_size].pos;
    }
    normal += normalize(cross(b - a, c - a));
    normal += normalize(cross(b - d, c - d));
    normal = normalize(normal);
    particles[x + y * cloth_size].norm = normal;
}

// -k(|d| - r) * (d/|d|) where d = p_1 - p_2
fn spring(p: Particle, np: Particle, r: f32, k: f32) -> vec3<f32> {
    let d = p.pos - np.pos;
    let mag = d/length(d);
    let f = -k * (length(d) - r) * mag;
    return f;
}

fn force_compute(gid: vec3<u32>) -> vec3<f32> {
    let p: Particle = particles[gid.x + gid.y * cloth_size];

    var forces: vec3<f32>;

    // gravity
    let f_gravity: f32 = -9.8;
    if (
        (gid.x == 0u && gid.y == 0u)
          || (gid.x == 0u && gid.y == cloth_size - 1u)
          || (gid.x == cloth_size - 1u && gid.y == 0u)
          || (gid.x == cloth_size - 1u && gid.y == cloth_size - 1u)
        // gid.x == 0u || gid.y == 0u || gid.x == cloth_size - 1u || gid.y == cloth_size - 1u
        ) {
        return vec3<f32>(0.0);
    }
    forces = vec3<f32>(0.0, f_gravity, 0.0);

    // wind
    let wind_dir = vec3(0.0, 0.0, -100.0);
    forces += wind_dir * abs(dot(p.norm, -normalize(wind_dir)));// * p.norm;

    // structural, shear, & flex springs
    var total_spring: vec3<f32>;

    // r for structural is 1.0
    // r for shear is sqrt(2)
    // r for flex is 2.0
    let k = 8000.0;
    if (gid.x != 0u) {
        total_spring += spring(p, particles[(gid.x - 1u) + gid.y * cloth_size], 1.0, k);
    }
    if (gid.y != 0u) {
        total_spring += spring(p, particles[gid.x + (gid.y - 1u) * cloth_size], 1.0, k);
    }
    if (gid.x != (cloth_size - 1u)) {
        total_spring += spring(p, particles[(gid.x + 1u) + gid.y * cloth_size], 1.0, k);
    }
    if (gid.y != (cloth_size - 1u)) {
        total_spring += spring(p, particles[gid.x + (gid.y + 1u) * cloth_size], 1.0, k);
    }
    if (gid.x != 0u && gid.y != 0u) {
        total_spring += spring(p, particles[(gid.x - 1u) + (gid.y - 1u) * cloth_size], 1.41421, k);
    }
    if (gid.x != 0u && gid.y != (cloth_size - 1u)) {
        total_spring += spring(p, particles[(gid.x - 1u) + (gid.y + 1u) * cloth_size], 1.41421, k);
    }
    if (gid.x != (cloth_size - 1u) && gid.y != 0u) {
        total_spring += spring(p, particles[(gid.x + 1u) + (gid.y - 1u) * cloth_size], 1.41421, k);
    }
    if (gid.x != (cloth_size - 1u) && gid.y != (cloth_size - 1u)) {
        total_spring += spring(p, particles[(gid.x + 1u) + (gid.y + 1u) * cloth_size], 1.41421, k);
    }
    if (gid.x > 1u) {
        total_spring += spring(p, particles[(gid.x - 2u) + gid.y * cloth_size], 2.0, k);
    }
    if (gid.y > 1u) {
        total_spring += spring(p, particles[gid.x + (gid.y - 2u) * cloth_size], 2.0, k);
    }
    if (gid.x < (cloth_size - 2u)) {
        total_spring += spring(p, particles[(gid.x + 2u) + gid.y * cloth_size], 2.0, k);
    }
    if (gid.y < (cloth_size - 2u)) {
        total_spring += spring(p, particles[gid.x + (gid.y + 2u) * cloth_size], 2.0, k);
    }

    forces += total_spring;

    return forces;
}

// X = X(t) for purposes of concision
//
// euler time integrator (very unstable):
// X(t + h) = X + hf(X, t)
//
// trapezoidal time integrator (unstable):
// f0 = f(X, t)
// f1 = f(X + hf0, t + h)
// X(t + h) = X + (h/2)(f0 + f1)
//
// rk4 time integrator (stable):
// f0 = f(X, t)
// f1 = f(X + h*f0/2, t + h/2)
// f2 = f(X + h*f1/2, t + h/2)
// f3 = f(X + h*f2,   t + h)
// X(t + h) = X + (h/6)(f0 + 2f1 + 2f2 + f3)

let h: f32 = 0.00001;

@compute
@workgroup_size(16, 16, 1)
fn cloth_euler(@builtin(global_invocation_id) gid: vec3<u32>) {
    let forces = force_compute(gid);
    particles[gid.x + gid.y * cloth_size].vel += forces * h;
    particles[gid.x + gid.y * cloth_size].pos += particles[gid.x + gid.y * cloth_size].vel;
    set_normal(gid);
}

@group(1) @binding(0) var<storage, read_write> next: array<Particle>;
@group(1) @binding(1) var<storage, read_write> forcesum: array<vec3<f32>>;

// p = X, p1, p2, p3, s
// calc f0 from p and store X + hf0/2 into p1 and add f0 to s; particles is p, next is p1
// calc f1 from p1 and store X + hf1/2 into p2 and add 2f1 to s; particles is p1, next is p2
// calc f2 from p2 and store X + hf2 into p3 and add 2f2 to s; particles is p2, next is p3
// calc f3 from p3 and add f3 to s; particles is p3, next is _
// add (h/6)s to p; particles is p, next is _ (also set the normals now that everything's been moved)
@compute
@workgroup_size(16, 16, 1)
fn rk4_1(@builtin(global_invocation_id) gid: vec3<u32>) {
    let forces = force_compute(gid);
    forcesum[gid.x + gid.y * cloth_size] += forces;
    next[gid.x + gid.y * cloth_size].vel = particles[gid.x + gid.y * cloth_size].vel + forces * h / 2.0;
    next[gid.x + gid.y * cloth_size].pos = particles[gid.x + gid.y * cloth_size].pos + next[gid.x + gid.y * cloth_size].vel;
}
@compute
@workgroup_size(16, 16, 1)
fn rk4_2(@builtin(global_invocation_id) gid: vec3<u32>) {
    let forces = force_compute(gid);
    forcesum[gid.x + gid.y * cloth_size] += forces * 2.0;
    next[gid.x + gid.y * cloth_size].vel = particles[gid.x + gid.y * cloth_size].vel + forces * h / 2.0;
    next[gid.x + gid.y * cloth_size].pos = particles[gid.x + gid.y * cloth_size].pos + next[gid.x + gid.y * cloth_size].vel;
}
@compute
@workgroup_size(16, 16, 1)
fn rk4_3(@builtin(global_invocation_id) gid: vec3<u32>) {
    let forces = force_compute(gid);
    forcesum[gid.x + gid.y * cloth_size] += forces * 2.0;
    next[gid.x + gid.y * cloth_size].vel = particles[gid.x + gid.y * cloth_size].vel + forces * h;
    next[gid.x + gid.y * cloth_size].pos += next[gid.x + gid.y * cloth_size].vel;
}
@compute
@workgroup_size(16, 16, 1)
fn rk4_4(@builtin(global_invocation_id) gid: vec3<u32>) {
    let forces = force_compute(gid);
    forcesum[gid.x + gid.y * cloth_size] += forces;
}
@compute
@workgroup_size(16, 16, 1)
fn rk4_5(@builtin(global_invocation_id) gid: vec3<u32>) {
    let s = forcesum[gid.x + gid.y * cloth_size];
    particles[gid.x + gid.y * cloth_size].vel += s * h / 6.0;
    particles[gid.x + gid.y * cloth_size].pos += particles[gid.x + gid.y * cloth_size].vel;
    forcesum[gid.x + gid.y * cloth_size] = vec3<f32>(0.0);

    // sets the normals
    set_normal(gid);
}

@group(0) @binding(0) var<storage, read> readonly_particles: array<Particle>;

struct PushConstants {
    projview: mat4x4<f32>,
    flag: u32,
}

var<push_constant> push_constants: PushConstants;

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) vtx: u32,
    @location(1) norm: vec3<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vtx: u32,
) -> VertexOutput {
    let p = readonly_particles[vtx];

    var vert: VertexOutput;
    vert.pos = push_constants.projview * vec4(p.pos, 1.0);
    vert.vtx = vtx;
    vert.norm = p.norm;
    return vert;
}

@fragment
fn fs_main(v_in: VertexOutput) -> @location(0) vec4<f32> {
    let tier = f32((cloth_size - 1u) - v_in.vtx / cloth_size) / f32(cloth_size);

    let t_a = max(dot(v_in.norm, -normalize(vec3(0.0, -1.0, -4.0))), 0.3);
    let t_b = max(dot(-v_in.norm, -normalize(vec3(0.0, -1.0, -4.0))), 0.3);
    let light_factor = max(t_a, t_b);

    if (push_constants.flag == 0u) {
        // rainbow pride :D
        if (tier < 0.1647) {
            return vec4<f32>(0.898, 0.0, 0.0, 1.0) * light_factor;
        } else if (tier < 0.333) {
            return vec4<f32>(1.0, 0.553, 0.0, 1.0) * light_factor;
        } else if (tier < 0.5) {
            return vec4<f32>(1.0, 0.933, 0.0, 1.0) * light_factor;
        } else if (tier < 0.666) {
            return vec4<f32>(0.007, 0.598, 0.0, 1.0) * light_factor;
        } else if (tier < 0.8314) {
            return vec4<f32>(0.0, 0.298, 1.0, 1.0) * light_factor;
        } else {
            return vec4<f32>(0.466, 0.0, 0.533, 1.0) * light_factor;
        }
    } else {
        // trans pride :D
        if (tier < 0.2) {
            return vec4<f32>(0.333, 0.804, 0.988, 1.0) * light_factor;
        } else if (tier < 0.4) {
            return vec4<f32>(0.968, 0.659, 0.722, 1.0) * light_factor;
        } else if (tier < 0.6) {
            return vec4<f32>(1.0, 1.0, 1.0, 1.0) * light_factor;
        } else if (tier < 0.8) {
            return vec4<f32>(0.968, 0.659, 0.722, 1.0) * light_factor;
        } else {
            return vec4<f32>(0.333, 0.804, 0.988, 1.0) * light_factor;
        }
    }
}