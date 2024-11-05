#version 440

layout(location = 0) out vec4 o_color;
layout(location = 0) in vec2 i_tex_coord;
layout(location = 1) in vec3 i_tex_coord_3d;

layout(set = 0, binding = 0) uniform sampler2D u_tex1;
layout(set = 0, binding = 1) uniform sampler2D u_tex2;

layout(set = 0, binding = 2) uniform sampler u_sam;
layout(set = 0, binding = 3) uniform texture2D u_tex;

layout(set = 0, binding = 4) uniform sampler2DArray u_tex_array;

void test_b(sampler2D b0, sampler2DArray b1) {
    vec4 res2 = texture(b0, i_tex_coord);
    o_color = res2;

    vec4 res3 = texture(b1, i_tex_coord_3d);
    o_color = res3;
}

void test_a(sampler2D a0, sampler2DArray a1) {
    test_b(a0, a1);
}

void main() {
    vec4 res1 = texture(u_tex1, i_tex_coord);
    vec4 res2 = texture(sampler2D(u_tex, u_sam), i_tex_coord);

    test_a(u_tex1, u_tex_array);
}

