#version 440

layout(location = 0) out vec4 o_color;
layout(location = 0) in vec2 i_tex_coord;

layout(set = 0, binding = 0) uniform sampler2D u_tex1;
layout(set = 0, binding = 1) uniform sampler2D u_tex2;

layout(set = 0, binding = 2) uniform sampler u_sam;
layout(set = 0, binding = 3) uniform texture2D u_tex;

void test_b(sampler2D b) {
    vec4 res2 = texture(b, i_tex_coord);
    o_color = res2;
}

void test_a(sampler2D a) {
    test_b(a);
}

void main() {
    vec4 res1 = texture(u_tex1, i_tex_coord);
    vec4 res2 = texture(sampler2D(u_tex, u_sam), i_tex_coord);

    test_a(u_tex1);
}

