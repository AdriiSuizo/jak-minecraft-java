#version 330
#extension GL_ARB_separate_shader_objects : require

#include <minecraft:fog.glsl>
#include <minecraft:dynamictransforms.glsl>
#include <minecraft:projection.glsl>

// Minecraft's core/entity transform, with the second texture coordinate read from the overlay slot:
// the envmap pass needs both the sphere-map coordinates (UV0) and the base's own (UV1, 1/4096 texel, offset 16384).
layout(location = 0) in vec3 Position;
layout(location = 2) in vec2 UV0;
layout(location = 3) in ivec2 UV1;

layout(location = 0) out float sphericalVertexDistance;
layout(location = 1) out float cylindricalVertexDistance;
layout(location = 6) out vec2 texCoord0;
layout(location = 7) out vec2 texCoord1;

void main() {
    gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);

    sphericalVertexDistance = fog_spherical_distance(Position);
    cylindricalVertexDistance = fog_cylindrical_distance(Position);

    texCoord0 = UV0;
    texCoord1 = (vec2(UV1) - 16384.0) / 4096.0;
}
