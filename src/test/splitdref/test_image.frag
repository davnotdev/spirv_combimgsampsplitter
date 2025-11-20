#version 450

layout(set = 0, binding = 0) uniform sampler u_regular_sampler;  // Regular sampler
layout(set = 0, binding = 1) uniform sampler u_comparison_sampler;  // Shadow map sampler

layout(set = 0, binding = 2) uniform texture2D u_mixed_texture;

layout(set = 0, binding = 3) uniform texture2D u_other_a;
layout(set = 0, binding = 4) uniform texture2D u_other_b;

void main() {
    float g0 = textureProj(sampler2DShadow(u_mixed_texture, u_comparison_sampler), vec4(0.0, 0.0, 0.0, 0.0));
    vec4 g1 = textureLod(sampler2D(u_mixed_texture, u_regular_sampler), vec2(0.0, 0.0), 0);
    vec4 g2 = textureLod(sampler2D(u_other_b, u_regular_sampler), vec2(0.0, 0.0), 0);
}

