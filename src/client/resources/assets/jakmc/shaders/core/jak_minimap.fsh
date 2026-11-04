#version 330
#extension GL_ARB_separate_shader_objects : require
layout(std140) uniform DynamicTransforms {
    mat4 ModelViewMat;
    mat4 TextureMat;
    vec4 ColorModulator;
    vec3 ModelOffset;
};

uniform sampler2D Sampler0;
layout(location = 0) in vec2 texCoord0;
layout(location = 1) in vec4 vertexColor;
layout(location = 0) out vec4 fragColor;

void main() {
    // Each quad encodes the crop centre and radius in its constant vertex colour.
    float distanceToEdge = length(texCoord0 - vertexColor.rg) / vertexColor.b;
    float mask = 1.0 - smoothstep(0.969, 1.0, distanceToEdge);
    if (mask == 0.0) discard;
    vec4 terrain = texture(Sampler0, texCoord0);
    fragColor = vec4(terrain.rgb * 0.90, terrain.a * mask) * ColorModulator;
}
