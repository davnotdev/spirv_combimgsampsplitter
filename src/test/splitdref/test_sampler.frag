#version 450

layout(set = 0, binding = 0) uniform texture2D u_regular_texture;
layout(set = 0, binding = 1) uniform texture2D u_comparison_texture;

layout(set = 0, binding = 2) uniform sampler u_mixed_sampler;

layout(set = 0, binding = 3) uniform texture2D u_other_texture;
layout(set = 0, binding = 4) uniform sampler u_other_sampler;

void main() {
    float g0 = textureProj(sampler2DShadow(u_comparison_texture, u_mixed_sampler), vec4(0.0, 0.0, 0.0, 0.0));
    vec4 g1 = textureLod(sampler2D(u_regular_texture, u_mixed_sampler), vec2(0.0, 0.0), 0);
    vec4 g2 = textureLod(sampler2D(u_other_texture, u_other_sampler), vec2(0.0, 0.0), 0);
}

