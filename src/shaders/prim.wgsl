struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VSOut {
    var tri = array<vec2<f32>, 3>(
        vec2<f32>(1.0, -3.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(-3.0, 1.0),
    );

    var out: VSOut;
    let newpos = tri[index];
    out.pos = vec4<f32>(newpos, 0.0, 1.0);

    out.uv = newpos * 0.5 + vec2<f32>(0.5, 0.5);
    return out;
}

struct Primitive {
    xyzw: vec4<f32>,
    r: f32,
    label: u32,
    _pad0: u32,
    _pad1: u32,
    color: vec4<f32>,
};

struct Primitives {
    count: u32,
    _pad: u32,
    scale: vec2<f32>,
    data: array<Primitive, 256>,
};

@group(0) @binding(0)
var<uniform> primitives: Primitives;

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    var color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    var coords = (uv) * primitives.scale;

    for (var i: u32 = 0u; i < primitives.count; i++) {
        
        let primitive = primitives.data[i];
        if (primitive.label == 0) {
            if (primitive.xyzw.x < coords.x) {
                if (primitive.xyzw.x + primitive.xyzw.z > coords.x) {
                    if (primitive.xyzw.y < coords.y) {
                        if (primitive.xyzw.y + primitive.xyzw.w > coords.y) {
                            color = primitive.color;
                        }
                    }
                }
            }
        } else if (primitive.label == 1) {
            let mult: f32 = clamp(primitive.r-distance(coords, primitive.xyzw.xy), 0.0, 1.0);
            color = color + (primitive.color - color)*mult;
        } else if (primitive.label == 2) {
            let ab: vec2<f32> = primitive.xyzw.zw-primitive.xyzw.xy;
            let ap: vec2<f32> = coords-primitive.xyzw.xy;
            let ln_pos: f32  = clamp(dot(ap, ab) / dot(ab, ab), 0, 1);
            let closest_point: vec2<f32> = primitive.xyzw.xy + ab*ln_pos;
            let mult: f32 = clamp(primitive.r-distance(coords, closest_point), 0.0, 1.0);
            color = color + (primitive.color - color)*mult;
        }
    }

    return color;
}