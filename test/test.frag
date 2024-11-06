#version 440

layout(location = 0) out vec4 o_color;
layout(location = 0) in vec2 i_tex_coord;

layout(set = 0, binding = 0) uniform sampler2D u_comb;

void main() {
    vec4 res = texture(u_comb, i_tex_coord);
    o_color = res;
}
