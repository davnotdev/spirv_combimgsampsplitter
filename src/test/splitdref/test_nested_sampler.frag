#version 450

layout(set = 0, binding = 0) uniform texture2D u_regular_texture;
layout(set = 0, binding = 1) uniform texture2D u_comparison_texture;

layout(set = 0, binding = 2) uniform sampler u_mixed_sampler;

layout(set = 0, binding = 3) uniform texture2D u_other_texture;
layout(set = 0, binding = 4) uniform sampler u_other_sampler;

void b(sampler mixed_sampler, texture2D regular_texture, texture2D comparison_texture, texture2D other_texture, sampler other_sampler) {
    float g0 = textureProj(sampler2DShadow(comparison_texture, mixed_sampler), vec4(0.0, 0.0, 0.0, 0.0));
    vec4 g1 = textureLod(sampler2D(regular_texture, mixed_sampler), vec2(0.0, 0.0), 0);
    vec4 g2 = textureLod(sampler2D(other_texture, other_sampler), vec2(0.0, 0.0), 0);
}

void a(sampler mixed_sampler, texture2D regular_texture, texture2D comparison_texture, texture2D other_texture, sampler other_sampler) {
    b(mixed_sampler, regular_texture, comparison_texture, other_texture, other_sampler);
}

void main() {
    a(u_mixed_sampler, u_regular_texture, u_comparison_texture, u_other_texture, u_other_sampler);
    b(u_mixed_sampler, u_regular_texture, u_comparison_texture, u_other_texture, u_other_sampler);
}


