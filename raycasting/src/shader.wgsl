// pad to reach 32 bits
struct Sphere {
    center: vec3<f32>,
    radius: f32,
    transform_index: u32,
    material: u32,
    _a: u32,
    _b: u32,
}

// pad to reach 32 bits
struct Plane {
    normal: vec3<f32>,
    offset: f32,
    material: u32,
    _a: u32,
    _b: u32,
}

struct Triangle {
    a: vec3<f32>,
    b: vec3<f32>,
    c: vec3<f32>,
}

// pad to reach 32
struct Mesh {
    vertex_offset: u32,
    index_offset: u32,
    index_count: u32,
    transform_idx: u32,
    material: u32,
    flat: u32,
    tcoords: u32,
    _c: u32,
}

struct Perspective {
    origin: vec3<f32>,
    horizontal: vec3<f32>,
    up: vec3<f32>,
    aspect: f32,
    direction: vec3<f32>,
    angle: f32,
}

struct Light {
    first: vec3<f32>,
    falloff: f32,
    color: vec3<f32>,
    // 0 if direction, 1 if point
    direction_or_point: u32,
}

struct Material {
    diffuse: vec3<f32>,
    // -1 indicates no texture
    texture: i32,
    specular: vec3<f32>,
    shininess: f32,
    refractive_index: f32,
    _a: f32,
    _b: f32,
    _c: f32,
}

@group(0) @binding(0) var out_tex: texture_storage_2d<rgba8unorm, write>;

@group(0) @binding(1) var<uniform> perspective: Perspective;

@group(0) @binding(2) var<uniform> spheres: array<Sphere, 64>;
@group(0) @binding(3) var<uniform> planes: array<Plane, 64>;

@group(0) @binding(4) var<storage, read> positions: array<vec3<f32>, 1024>;
@group(0) @binding(10) var<storage, read> normals: array<vec3<f32>, 1024>;
@group(0) @binding(11) var<storage, read> texture_coords: array<vec2<f32>, 1024>;
@group(0) @binding(5) var<storage, read> indices: array<u32, 2048>;
@group(0) @binding(6) var<uniform> meshes: array<Mesh, 64>;

@group(0) @binding(12) var tex: binding_array<texture_2d<f32>>;
@group(0) @binding(13) var sam: sampler;

@group(0) @binding(14) var cubemap: texture_cube<f32>;

// 0th transform is identity
@group(0) @binding(7) var<uniform> transforms: array<mat4x4<f32>, 64>;

@group(0) @binding(8) var<uniform> materials: array<Material, 64>;

@group(0) @binding(9) var<uniform> lights: array<Light, 64>;

struct PushConstants {
    // vec4 so i don't forget alignment :)
    bg_color: vec4<f32>,
    screensize: vec2<f32>,
    sphere_cts: u32,
    plane_cts: u32,
    mesh_cts: u32,
    light_cts: u32,
    max_bounces: u32,
    has_cubemap: u32,
    ambient_light: vec4<f32>,
}

var<push_constant> pcs: PushConstants;

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
}

fn gen_ray(uv: vec2<f32>) -> Ray {
    var ray: Ray;

    // see perspective.rs
    ray.origin = perspective.origin;
    let x = uv.x * perspective.aspect;
    ray.direction = normalize(perspective.direction + (x / 2.0) * perspective.horizontal + (uv.y / 2.0) * perspective.up);

    return ray;
}

// relevant material: https://gitlab.com/willp-public/gpu-ray-tracing-post

// https://en.wikipedia.org/wiki/Line%E2%80%93sphere_intersection
fn hit_sphere(ray: Ray, sphere: Sphere) -> f32 {
    let dist = ray.origin - sphere.center;
    let a = dot(ray.direction, ray.direction);
    let b = dot(dist, ray.direction);
    let c = dot(dist, dist) - sphere.radius * sphere.radius;
    let d = b * b - a * c;
    if (d > 0.0) {
        let sqd = sqrt(d);
        let fnl = (-b - sqd) / a;
        return fnl;
    }
    // dummy fail value
    return -1.0;
}

// https://en.wikipedia.org/wiki/Line%E2%80%93plane_intersection
fn hit_plane(ray: Ray, plane: Plane) -> f32 {
    let dist = dot((ray.origin + (plane.offset * plane.normal)), plane.normal);
    let ln = dot(ray.direction, plane.normal);
    if (ln == 0.0) {
        // line & plane are parallel;
        // dummy value, negative values are in fact possible
        // but that means the plane is intersected behind the ray
        // so we ignore all negative values
        return -1.0;
    }
    let d = dist/ln;
    return d;
}

// (part of hit_triangle)
fn same_side(a: vec3<f32>, b: vec3<f32>, c: vec3<f32>, d: vec3<f32>, p: vec3<f32>) -> bool {
    let normal = cross(b - a, c - a);
    let dotd = dot(normal, d - a);
    let dotp = dot(normal, p - a);
    return sign(dotd) == sign(dotp);
}

// https://stackoverflow.com/a/74231820
// (cited by the above answer) https://stackoverflow.com/a/25180294
// not the most efficient solution but easy to implement
fn hit_triangle(ray: Ray, tri: Triangle) -> f32 {
    if (same_side(tri.a, tri.b, tri.c, ray.origin, ray.origin + ray.direction) &&
        same_side(tri.b, tri.c, ray.origin, tri.a, ray.origin + ray.direction) &&
        same_side(tri.c, ray.origin, tri.a, tri.b, ray.origin + ray.direction) &&
        same_side(ray.origin, tri.a, tri.b, tri.c, ray.origin + ray.direction)) {
        let n = cross(tri.b - tri.a, tri.c - tri.a);
        let t = dot(tri.a - ray.origin, n) / dot(ray.direction, n);
        return t;
    } else {
        // dummy fail value
        return -1.0;
    }
}

// https://stackoverflow.com/a/22217694 led me to
// https://answers.unity.com/questions/383804/calculate-uv-coordinates-of-3d-point-on-plane-of-m.html
// the answer the unity forum post provided was very helpful and I basically copied it
// the vec3 contains the weights for t.a, t.b, and t.c in the x, y, and z fields respectively
fn barycentric_interpolate(t: Triangle, loc: vec3<f32>) -> vec3<f32> {
    let f1 = t.a - loc;
    let f2 = t.b - loc;
    let f3 = t.c - loc;
    let a = length(cross(t.a - t.b, t.a - t.c));
    let a1 = length(cross(f2, f3)) / a;
    let a2 = length(cross(f3, f1)) / a;
    let a3 = length(cross(f1, f2)) / a;
    return vec3(a1, a2, a3);
}

struct Intersection {
    distance: f32,
    object: bool,
    object_material: Material,
    object_normal: vec3<f32>,
    object_texture_coords: vec2<f32>,
    tcoords_index_plus_one: u32,
}

fn check_intersection(ray: Ray, tmin: f32) -> Intersection {
    var ints: Intersection;

    var i: u32;
    var m: f32;
    var mc: u32;
    var tray: Ray;

    m = 10000.0;
    loop {
        if (i >= pcs.sphere_cts) {
            break;
        }
        tray.origin = (transforms[spheres[i].transform_index] * vec4(ray.origin, 1.0)).xyz;
        tray.direction = (transforms[spheres[i].transform_index] * vec4(ray.direction, 1.0)).xyz;
        let h = hit_sphere(tray, spheres[i]);
        if (h > tmin && h < m) {
            m = h;
            ints.object_material = materials[spheres[i].material];
            ints.object = true;
            let intersection = h * tray.direction + tray.origin;
            ints.object_normal = (intersection - spheres[i].center) / spheres[i].radius;
        }
        ints.tcoords_index_plus_one = 0u;
        continuing {
            i = i + 1u;
        }
    }
    i = 0u;
    loop {
        if (i >= pcs.plane_cts) {
            break;
        }
        let h = hit_plane(ray, planes[i]);
        if (h > tmin && h < m) {
            m = h;
            ints.object_material = materials[planes[i].material];
            ints.object_normal = planes[i].normal;
            ints.object = true;
        }
        ints.tcoords_index_plus_one = 0u;
        continuing {
            i = i + 1u;
        }
    }
    i = 0u;
    loop {
        if (i >= pcs.mesh_cts) {
            break;
        }
        let mesh = meshes[i];
        mc = 0u;

        let transform = transforms[mesh.transform_idx];

        loop {
            if (mc >= mesh.index_count) {
                break;
            }
            let v1 = positions[indices[mc + mesh.index_offset] + mesh.vertex_offset];
            mc = mc + 1u;
            let v2 = positions[indices[mc + mesh.index_offset] + mesh.vertex_offset];
            mc = mc + 1u;
            let v3 = positions[indices[mc + mesh.index_offset] + mesh.vertex_offset];
            mc = mc + 1u;
            let a1 = (transform * vec4(v1, 1.0)).xyz;
            let a2 = (transform * vec4(v2, 1.0)).xyz;
            let a3 = (transform * vec4(v3, 1.0)).xyz;
            var t: Triangle;
            t.a = a1;
            t.b = a2;
            t.c = a3;
            let h = hit_triangle(ray, t);
            if (h > tmin && h < m) {
                m = h;
                ints.object_material = materials[mesh.material];
                ints.object = true;

                // interpolate for texture coords & normals
                let loc = ray.direction * h + ray.origin;
                let weights = barycentric_interpolate(t, loc);

                // normal calculation
                if (mesh.flat == 0u) {
                    // smooth shading
                    let n1 = normalize((transform * vec4(normals[indices[mc - 3u + mesh.index_offset] + mesh.vertex_offset], 0.0)).xyz);
                    let n2 = normalize((transform * vec4(normals[indices[mc - 2u + mesh.index_offset] + mesh.vertex_offset], 0.0)).xyz);
                    let n3 = normalize((transform * vec4(normals[indices[mc - 1u + mesh.index_offset] + mesh.vertex_offset], 0.0)).xyz);
                    ints.object_normal = n1 * weights.x + n2 * weights.y + n3 * weights.z;
                } else {
                    // flat shading
                    ints.object_normal = normalize(cross(t.b - t.a, t.c - t.b));
                }

                // texture coordinate stuff
                if (mesh.tcoords != 0u) {
                    let t1 = texture_coords[indices[mc - 3u + mesh.index_offset] + mesh.vertex_offset];
                    let t2 = texture_coords[indices[mc - 2u + mesh.index_offset] + mesh.vertex_offset];
                    let t3 = texture_coords[indices[mc - 1u + mesh.index_offset] + mesh.vertex_offset];
                    ints.object_texture_coords = t1 * weights.x + t2 * weights.y + t3 * weights.z;
                    ints.tcoords_index_plus_one = mesh.tcoords;
                } else {
                    ints.tcoords_index_plus_one = 0u;
                }
            }
        }
        continuing {
            i = i + 1u;
        }
    }

    ints.distance = m;

    return ints;
}

fn cubemap_color(dir: vec3<f32>) -> vec3<f32> {
    var color: vec3<f32>;
    if (abs(dir.x) >= abs(dir.y) && abs(dir.x) >= abs(dir.z)) {
        if (dir.x > 0.0) {
            let x = 1.0 - (dir.z / dir.x + 1.0) * 0.5;
            let y = (dir.y / dir.x + 1.0) * 0.5;
            color = textureSampleLevel(cubemap, sam, vec3(-1.0, y * 2.0 - 1.0, x * 2.0 - 1.0), 0.0).rgb;
        } else {
            let x = (dir.z / dir.x + 1.0) * 0.5;
            let y = 1.0 - (dir.y / dir.x + 1.0) * 0.5;
            color = textureSampleLevel(cubemap, sam, vec3(1.0, y * 2.0 - 1.0, x * 2.0 - 1.0), 0.0).rgb;
        }
    } else if (abs(dir.y) >= abs(dir.x) && abs(dir.y) >= abs(dir.z)) {
        if (dir.y > 0.0) {
            let x = 1.0 - (dir.x / dir.y + 1.0) * 0.5;
            let y = 1.0 - (dir.z / dir.y + 1.0) * 0.5;
            color = textureSampleLevel(cubemap, sam, vec3(x * 2.0 - 1.0, 1.0, y * 2.0 - 1.0), 0.0).rgb;
        } else {
            let x = 1.0 - (dir.x / dir.y + 1.0) * 0.5;
            let y = 1.0 - (dir.z / dir.y + 1.0) * 0.5;
            color = textureSampleLevel(cubemap, sam, vec3(x * 2.0 - 1.0, -1.0, y * 2.0 - 1.0), 0.0).rgb;
        }
    } else if (abs(dir.z) >= abs(dir.x) && abs(dir.z) >= abs(dir.y)) {
        if (dir.z > 0.0) {
            let x = 1.0 - (dir.x / dir.z + 1.0) * 0.5;
            let y = (dir.y / dir.z + 1.0) * 0.5;
            color = textureSampleLevel(cubemap, sam, vec3(x * 2.0 - 1.0, y * 2.0 - 1.0, -1.0), 0.0).rgb;
        } else {
            let x = (dir.x / dir.z + 1.0) * 0.5;
            let y = 1.0 - (dir.y / dir.z + 1.0) * 0.5;
            color = textureSampleLevel(cubemap, sam, vec3(x * 2.0 - 1.0, y * 2.0 - 1.0, 1.0), 0.0).rgb;
        }
    }
    return color;
}

struct RayTest {
    color: vec3<f32>,
    stop: bool,
    intersection: Intersection,
}

fn color(ray: Ray, shadows: bool) -> RayTest {
    var c: vec3<f32>;
    var i: u32;

    c = pcs.bg_color.rgb;

    let ints = check_intersection(ray, 0.0);

    if (!ints.object) {
        if (pcs.has_cubemap == 1u) {
            c = cubemap_color(normalize(ray.direction));
        }
        var r: RayTest;
        r.color = c;
        r.stop = true;
        return r;
    }

    var underlying_color: vec3<f32>;
    if (ints.tcoords_index_plus_one == 0u) {
        underlying_color = ints.object_material.diffuse;
    } else {
        underlying_color = textureSampleLevel(tex[ints.tcoords_index_plus_one - 1u], sam, ints.object_texture_coords, 0.0).rgb;
        // underlying_color = textureSampleLevel(tex, sam, ints.object_texture_coords, i32(ints.tcoords_index_plus_one - 1u), 0.0).rgb;
    }

    c = pcs.ambient_light.xyz * underlying_color;
    
    i = 0u;
    loop {
        if (i >= pcs.light_cts) {
            break;
        }
        let light = lights[i];

        var intersection_to_light: vec3<f32>;
        if (light.direction_or_point == 0u) {
            // direction
            intersection_to_light = -light.first;
        } else {
            // point
            intersection_to_light = normalize(light.first - (ray.direction * ints.distance + ray.origin));
        }

        if (shadows) {
            // shadows
            var r: Ray;
            r.origin = ray.direction * ints.distance + ray.origin;
            r.direction = intersection_to_light;//normalize(light.first - r.origin);
            let shadowintersect = check_intersection(r, 0.01);
            var ldist: f32;
            if (light.direction_or_point == 0u) {
                ldist = 10000.0;
            } else {
                ldist = length(r.origin - light.first);
            }
            if (shadowintersect.object && ldist > shadowintersect.distance) {
                continue;
            }
        }
        c += max(dot(intersection_to_light, ints.object_normal), 0.0) * light.color * underlying_color;
        if (ints.object_material.shininess != 0.0 && pcs.max_bounces == 0u) {
            let specular = dot(reflect(intersection_to_light, ints.object_normal), ray.direction);
            c += pow(max(specular, 0.0), ints.object_material.shininess) * ints.object_material.specular * light.color;
        }

        continuing {
            i = i + 1u;
        }
    }

    var out: RayTest;
    out.color = c;
    out.stop = false;
    out.intersection = ints;
    return out;
}

fn transmitted(normal: vec3<f32>, incoming: vec3<f32>, index_n: f32, index_nt: f32) -> vec3<f32> {
    let s = 1.0 - (index_n * index_n * (1.0 - dot(incoming, normal) * dot(incoming, normal))) / (index_nt * index_nt);
    if (s > 0.0) {
        return (index_n * (incoming - normal * dot(incoming, normal)) / index_nt) - normal * sqrt(s);
    } else {
        return vec3(0.0, 0.0, 0.0);
    }
}

@compute
@workgroup_size(16, 16, 1)
fn render(@builtin(global_invocation_id) gid: vec3<u32>) {
    // normalize to screenspace coords;
    // bl [-1, -1], tl [-1, 1], br [1, -1], tr [1, 1]
    let s_x = f32(gid.x) * 2.0 / pcs.screensize.x - 1.0;
    let s_y = -f32(gid.y) * 2.0 / pcs.screensize.y + 1.0;
    let uv = vec2(s_x, s_y);

    var ray: Ray;
    ray = gen_ray(uv);

    var col: vec3<f32>;
    var spec: vec3<f32>;
    var hit: RayTest;

    hit = color(ray, true);
    col = hit.color;
    spec = hit.intersection.object_material.specular;

    var i: u32;
    i = pcs.max_bounces;

    if (!hit.stop && hit.intersection.object_material.refractive_index == 0.0) {
        loop {
            if (i == 0u || hit.stop) {
                break;
            }
            var r: Ray;
            r.origin = ray.direction * hit.intersection.distance + ray.origin;
            r.direction = reflect(normalize(r.origin - ray.origin), hit.intersection.object_normal);
            hit = color(r, true);
            col += spec * hit.color;
            spec *= hit.intersection.object_material.specular;
            ray = r;
            continuing {
                i -= 1u;
            }
        }
    }

    // refraction calculations
    // if (hit.intersection.object_material.refractive_index != 0.0) {
    //     if (!hit.stop) {
    //         let dir = transmitted(hit.intersection.object_normal, ray, 1.0, hit.intersection.object_material.refractive_index);
    //     }
    // }

    // normalized to [0, 1]; testing only
    // let test = vec4<f32>((s_x + 1.0) / 2.0, (s_y + 1.0) / 2.0, 0.0, 1.0);

    textureStore(out_tex, vec2(i32(gid.x), i32(gid.y)), vec4(col, 1.0));
}
