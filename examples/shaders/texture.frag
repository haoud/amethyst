#version 460

layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 out_color;

layout(binding = 1) uniform sampler2D texture_sampler;

void main() {
    out_color = texture(texture_sampler, uv);
}