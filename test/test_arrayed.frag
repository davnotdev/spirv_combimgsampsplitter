#version 440

layout(location = 0) out vec4 o_color;
layout(location = 0) in vec3 i_tex_coord;

layout(set = 0, binding = 0) uniform sampler2DArray u_comb;

void main() {
    o_color = texture(u_comb, i_tex_coord);
}

