#version 450

layout(set = 0, binding = 0) uniform sampler u_regular_sampler;
layout(set = 0, binding = 1) uniform sampler u_comparison_sampler;

layout(set = 0, binding = 2) uniform texture2D u_mixed_texture;

layout(set = 0, binding = 3) uniform texture2D u_other_a;
layout(set = 0, binding = 4) uniform texture2D u_other_b;

void a(uint useless, texture2D mixed_texture, sampler regular_sampler, sampler comparison_sampler, texture2D other_b) {
    float g0 = textureProj(sampler2DShadow(mixed_texture, comparison_sampler), vec4(0.0, 0.0, 0.0, 0.0));
    vec4 g1 = textureLod(sampler2D(mixed_texture, regular_sampler), vec2(0.0, 0.0), 0);
    vec4 g2 = textureLod(sampler2D(other_b, regular_sampler), vec2(0.0, 0.0), 0);
}

void b(sampler regular_sampler, sampler comparison_sampler, texture2D other_b, texture2D mixed_texture) {
    a(0, mixed_texture, regular_sampler, comparison_sampler, other_b);
}

void c(sampler comparison_sampler, texture2D other_b, texture2D mixed_texture, sampler regular_sampler) {
    b(regular_sampler, comparison_sampler, other_b, mixed_texture);
}


void d(texture2D other_b, texture2D mixed_texture, sampler regular_sampler, sampler comparison_sampler) {
    c(comparison_sampler, other_b, mixed_texture, regular_sampler);
}

void e(texture2D mixed_texture, sampler regular_sampler, sampler comparison_sampler, texture2D other_b) {
    d(other_b, mixed_texture, regular_sampler, comparison_sampler);
}

void main() {
    e(u_mixed_texture, u_regular_sampler, u_comparison_sampler, u_other_b);
    d(u_other_b, u_mixed_texture, u_regular_sampler, u_comparison_sampler);
    c(u_comparison_sampler, u_other_b, u_mixed_texture, u_regular_sampler);
    b(u_regular_sampler, u_comparison_sampler, u_other_b, u_mixed_texture);
    a(0, u_mixed_texture, u_regular_sampler, u_comparison_sampler, u_other_b);
}

