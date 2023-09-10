#version 460

layout(location = 0) in vec3 position;
layout(location = 1) in vec2 tcoord;

layout(location = 0) out vec2 uvs;

layout(binding = 0) uniform UniformBufferObject {
    mat4 projection;
    mat4 model;
    mat4 view;
} ubo;

void main() {
    mat4 mpv = ubo.projection * ubo.view * ubo.model;
    gl_Position = mpv * vec4(position, 1.0);
    uvs = tcoord;
}