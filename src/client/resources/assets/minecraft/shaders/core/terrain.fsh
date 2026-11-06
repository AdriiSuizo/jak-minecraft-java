#version 330
#extension GL_ARB_separate_shader_objects : require

#include <minecraft:fog.glsl>
#include <minecraft:globals.glsl>
#include <minecraft:texture_sampling.glsl>
#include <minecraft:oit.glsl>
#include <minecraft:terrainglobals.glsl>
#include <jakmc:camera_mask.glsl>
#ifndef MULTIDRAW_TERRAIN
    #include <minecraft:chunksection.glsl>
#endif

uniform sampler2D Sampler0;

layout(location = 0) in float sphericalVertexDistance;
layout(location = 1) in float cylindricalVertexDistance;
layout(location = 2) in vec4 vertexColor;
layout(location = 3) in vec2 texCoord0;
layout(location = 4) in float chunkVisibility;
layout(location = 5) in vec3 jakSurface;

#ifndef OIT_ALPHA_ONLY
layout(location = 0) out vec4 fragColor;
#endif

// Jak's third-person camera: occlusion mask and room cutaway (jakmc:camera_mask.glsl). The terrain mask
// keeps the floor and the lowest 0.45 of walls over Jak's feet, so a cut never opens the hidden top
// face of the block under a wall.
float jak_terrain_cut(vec3 p) {
    float wallFoot = smoothstep(JakMask1.w + 0.45, JakMask1.w + 0.65, p.y);
    return max(jak_mask_cut(p, JakMask0.w, JakMask3.z) * wallFoot, jak_room_cut(p));
}

vec4 calculateFinalColor(vec4 color) {
    #ifdef OIT_ACCUMULATE
    color = sampleColorForAccumulation(color);
    vec4 fogColor = vec4(FogColor.rgb * color.a, FogColor.a);
    #else
    vec4 fogColor = FogColor;
    #endif
    return apply_fog(color, sphericalVertexDistance, cylindricalVertexDistance, FogEnvironmentalStart, FogEnvironmentalEnd, FogRenderDistanceStart, FogRenderDistanceEnd, fogColor);
}

void main() {
    float jakOpacity = 1.0 - jak_terrain_cut(jakSurface);
    // Opaque and cutout layers: dithered discard. The translucent layer (ALPHA_CUTOUT 0.1)
    // multiplies its alpha instead.
    bool jakTranslucent = false;
    #ifdef ALPHA_CUTOUT
    jakTranslucent = ALPHA_CUTOUT < 0.3;
    #endif
    if (!jakTranslucent && !jak_fade_keep(jakOpacity, gl_FragCoord.xy)) {
        discard;
    }
    vec4 color = (UseRgss == 1 ? sampleRGSS(Sampler0, texCoord0, 1.0f / TextureSize) : sampleNearest(Sampler0, texCoord0, 1.0f / TextureSize)) * vertexColor;
    if (jakTranslucent) {
        color.a *= jakOpacity;
        if (color.a <= 0.001) {
            discard;
        }
    }
    #ifndef OIT_ALPHA_ONLY
    color = mix(FogColor * vec4(1, 1, 1, color.a), color, chunkVisibility);
    #endif
    #ifdef ALPHA_CUTOUT
    if (color.a < ALPHA_CUTOUT) {
        discard;
    }
    #endif

    #ifdef OIT_ALPHA_ONLY
    executeAlphaOnlyPhase(gl_FragCoord.z, color.a);
    #else
    fragColor = calculateFinalColor(color);
    #endif
}
