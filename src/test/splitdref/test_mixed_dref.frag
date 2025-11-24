#version 450

layout(set = 0, binding = 0) uniform sampler u_sampler;
layout(set = 0, binding = 1) uniform texture2D u_texture;
layout(set = 0, binding = 2) uniform texture2D useless;

void a(sampler s, texture2D t) {
    float g2 = textureProj(sampler2DShadow(t, s), vec4(0.0, 0.0, 0.0, 0.0));
    vec4 g3 = textureLod(sampler2D(t, s), vec2(0.0, 0.0), 0);
}

void b(sampler s, texture2D t) {
    a(s, t);
}

void main() {

    a(u_sampler, u_texture);
    b(u_sampler, u_texture);

    float g0 = textureProj(sampler2DShadow(u_texture, u_sampler), vec4(0.0, 0.0, 0.0, 0.0));
    vec4 g1 = textureLod(sampler2D(u_texture, u_sampler), vec2(0.0, 0.0), 0);
}

