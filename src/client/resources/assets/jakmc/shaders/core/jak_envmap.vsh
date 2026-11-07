#version 330
#extension GL_ARB_separate_shader_objects : require

#include <minecraft:fog.glsl>
#include <minecraft:dynamictransforms.glsl>
#include <minecraft:projection.glsl>
#include <jakmc:emerc.glsl>

// Minecraft's core/entity transform, with the second texture coordinate read from the overlay slot:
// the envmap pass needs both emerc's coordinates and the base's own (UV1, 1/4096 texel, offset 16384); its colour is the amount.
layout(location = 0) in vec3 Position;
layout(location = 1) in vec4 Color;
layout(location = 3) in ivec2 UV1;
layout(location = 5) in vec3 Normal;

layout(location = 0) out float sphericalVertexDistance;
layout(location = 1) out float cylindricalVertexDistance;
layout(location = 6) out vec2 texCoord0;
layout(location = 7) out vec2 texCoord1;
layout(location = 8) out vec3 shineAmount;

void main() {
    gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);

    sphericalVertexDistance = fog_spherical_distance(Position);
    cylindricalVertexDistance = fog_cylindrical_distance(Position);

    texCoord0 = jak_emerc_st(ModelViewMat, Position, Normal);
    shineAmount = Color.rgb;
    texCoord1 = (vec2(UV1) - 16384.0) / 4096.0;
}
