
struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] tex_coords: vec2<f32>;
};

struct VertexInput {
    [[location(0)]] position: vec2<f32>;
    [[location(1)]] tex_coords: vec2<f32>;
};
struct UniformData {
    translate: vec2<f32>;
};

fn preprocess_position(in: vec2<f32>) -> vec2<f32> {
    return (in - 0.5) * 2.0;
}


[[group(0), binding(2)]]
var<uniform> uniform_data: UniformData;

[[stage(vertex)]]
fn font_vs_main(
    input: VertexInput
) -> VertexOutput {
    var out: VertexOutput;
    var inp_pos = preprocess_position(input.position);
    inp_pos = inp_pos * 1.0;
    inp_pos = inp_pos + uniform_data.translate / 4.0;
    out.position =  vec4<f32>(inp_pos, 0.0, 1.0);
    out.tex_coords = input.tex_coords;

    // out.tex_coords.x = out.tex_coords.x / 10.0;
    return out;
}

[[group(0), binding(0)]]
var texture: texture_2d<f32>;

[[group(0), binding(1)]]
var sampl: sampler;


[[stage(fragment)]]
fn font_fs_main(in_var: VertexOutput) -> [[location(0)]] vec4<f32> {
    var coords = in_var.tex_coords;
    let sampled = textureSample(texture, sampl, coords).zyx;
    let result: vec4<f32> =  vec4<f32>(1.0 - sampled, sampled.x + sampled.y + sampled.z);
    return result;
}



// Rectangle pass
struct RectVInput {
    [[location(0)]] position: vec2<f32>;
    [[location(1)]] color: u32;
};
struct RectVOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] color: vec4<f32>;
};
[[stage(vertex)]]
fn rect_vs_main(in_var: RectVInput) ->  RectVOutput {
    var out: RectVOutput;

    out.position = vec4<f32>(preprocess_position(in_var.position), 0.0, 1.0);
    out.color = unpack4x8unorm(in_var.color);
    return out;
}

[[stage(fragment)]]
fn rect_fs_main(in_var: RectVOutput) -> [[location(0)]] vec4<f32> {
    return in_var.color.xyzw;
}