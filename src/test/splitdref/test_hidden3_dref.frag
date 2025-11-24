#version 450

layout(set = 0, binding = 0) uniform sampler u_regular_sampler;
layout(set = 0, binding = 1) uniform sampler u_comparison_sampler;
layout(set = 0, binding = 2) uniform texture2D u_mixed_texture;

void confuse(sampler s, texture2D t) {
    float g2 = textureProj(sampler2DShadow(t, s), vec4(0.0, 0.0, 0.0, 0.0));
}

void main() {
    float g0 = textureProj(sampler2DShadow(u_mixed_texture, u_comparison_sampler), vec4(0.0, 0.0, 0.0, 0.0));
    vec4 g1 = textureLod(sampler2D(u_mixed_texture, u_regular_sampler), vec2(0.0, 0.0), 0);
    confuse(u_regular_sampler, u_mixed_texture);
}

