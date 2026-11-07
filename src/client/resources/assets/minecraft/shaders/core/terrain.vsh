#version 330
#extension GL_ARB_separate_shader_objects : require

#include <minecraft:fog.glsl>
#include <minecraft:globals.glsl>
#include <minecraft:projection.glsl>
#include <minecraft:sample_lightmap.glsl>
#include <minecraft:terrainglobals.glsl>
#include <jakmc:dark_eco.glsl>
#ifndef MULTIDRAW_TERRAIN
    #include <minecraft:chunksection.glsl>
#endif

layout(location = 0) in vec3 Position;
layout(location = 1) in vec4 Color;
layout(location = 2) in vec2 UV0;
layout(location = 3) in ivec2 UV2;
#ifdef MULTIDRAW_TERRAIN
layout(location = 4) in ivec3 ChunkPosition;
layout(location = 5) in float ChunkVisibility;
#endif

#ifndef OIT_ALPHA_ONLY
uniform sampler2D Sampler2;
#endif

layout(location = 0) out float sphericalVertexDistance;
layout(location = 1) out float cylindricalVertexDistance;
layout(location = 2) out vec4 vertexColor;
layout(location = 3) out vec2 texCoord0;
layout(location = 4) out float chunkVisibility;
// Jak's camera masks: the surface point, camera-relative (jakmc:camera_mask.glsl).
layout(location = 5) out vec3 jakSurface;
// Dark eco's faces: 1 on them, and the surface point in the world (jakmc:dark_eco.glsl).
layout(location = 6) out float jakEco;
layout(location = 7) out vec3 jakWorld;

void main() {
    vec3 pos = Position + (ChunkPosition - CameraBlockPos) + CameraOffset;
    jakWorld = Position + vec3(ChunkPosition);
    jakEco = jak_dark_eco_face(Color) ? 1.0 : 0.0;
    // Its top rides the waves (a top face is the one Minecraft leaves at full brightness).
    if (jakEco > 0.5 && Color.r > 0.95) {
        float jakRise = jak_dark_eco_wave(jakWorld.xz) - 0.18;
        pos.y += jakRise;
        jakWorld.y += jakRise;
    }
    gl_Position = ProjMat * ModelViewMat * vec4(pos, 1.0);
    jakSurface = pos;

    sphericalVertexDistance = fog_spherical_distance(pos);
    cylindricalVertexDistance = fog_cylindrical_distance(pos);
    #ifndef OIT_ALPHA_ONLY
    vertexColor = Color * sample_lightmap(Sampler2, UV2);
    #else
    vertexColor = Color;
    #endif
    texCoord0 = UV0;

    const float chunkFullyVisibleRange = 16.0;
    float dist = length(pos);
    chunkVisibility = mix(1.0, ChunkVisibility, clamp((dist - chunkFullyVisibleRange) / chunkFullyVisibleRange, 0.0, 1.0));
}
