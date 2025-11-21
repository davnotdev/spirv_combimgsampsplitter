#version 450

layout(set = 0, binding = 0) uniform sampler u_regular_sampler;
layout(set = 0, binding = 1) uniform sampler u_comparison_sampler;
layout(set = 0, binding = 2) uniform texture2D u_mixed_texture;

void a(sampler s, texture2D t) {
    float g0 = textureProj(sampler2DShadow(t, s), vec4(0.0, 0.0, 0.0, 0.0));
    vec4 g1 = textureLod(sampler2D(t, s), vec2(0.0, 0.0), 0);
}

void b(sampler s, texture2D t) {
    float g1 = textureProj(sampler2DShadow(t, s), vec4(0.0, 0.0, 0.0, 0.0));
}

void c(sampler s, texture2D t) {
    vec4 g2 = textureLod(sampler2D(t, s), vec2(0.0, 0.0), 0);
}

void d(sampler2D s, texture2D t) {
    c(s, t)
}

void main() {
    a(u_regular_sampler, u_mixed_texture);
    b(u_comparison_sampler, u_mixed_texture);
    c(u_regular_sampler, u_mixed_texture);
    d(u_regular_sampler, u_mixed_texture);
}

