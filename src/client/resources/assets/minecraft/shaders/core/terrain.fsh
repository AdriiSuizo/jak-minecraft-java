#version 330
#extension GL_ARB_separate_shader_objects : require

#include <minecraft:fog.glsl>
#include <minecraft:globals.glsl>
#include <minecraft:texture_sampling.glsl>
#include <minecraft:oit.glsl>
#include <minecraft:terrainglobals.glsl>
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

// Jak camera cutaway, ported from the Rust mashup's CAMERA_FADE_WGSL
// (crates/render_gpu/src/drawsurf/minecraft_world.rs). Map units become blocks
// at 25 units a metre and 4096/6400 blocks a metre: 2 units = 0.0512, 18 = 0.4608,
// 1 = 0.0256, 0.05 = 0.00128. JakFadeEye.w is the opening (Rust half_width / 50).
// stand-in: view_backface_hidden is not needed; Minecraft's terrain pipelines already cull every reverse face.
// view_opacity: 1 where the surface stays, 0 where it is cut away.
float jak_view_opacity(vec3 p) {
    if (JakFadeEye.w <= 0.0) return 1.0;
    float feet = JakFadeTarget.w;
    if (p.y < feet + 0.0512) return 1.0;
    vec3 eye = JakFadeEye.xyz;
    vec3 line = JakFadeTarget.xyz - eye;
    float span = max(length(line), 0.0256);
    vec3 ahead = line / span;
    float along = dot(p - eye, ahead);
    float stop = span + 0.4608;
    if (along < 0.0 || along > stop) return 1.0;
    float across = length(p - eye - ahead * along);
    float opening = clamp(JakFadeEye.w, 0.0, 1.0);
    float radius = along * 0.42 * opening;
    float soft = max(radius * 0.02, 0.00128);
    float sideways = smoothstep(radius - soft, radius, across);
    float lengthwise = smoothstep(stop - soft, stop, along);
    return max(sideways, lengthwise);
}

// camera_fade_keep: a 4x4 ordered dither cuts partially faded surfaces without sorting.
bool jak_fade_keep(float opacity, vec2 pixel) {
    if (opacity >= 0.999) return true;
    if (opacity <= 0.001) return false;
    int x = int(pixel.x) & 3;
    int y = int(pixel.y) & 3;
    const float bayer[16] = float[16](0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0);
    return opacity > (bayer[y * 4 + x] + 0.5) / 16.0;
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
    float jakOpacity = jak_view_opacity(jakSurface);
    // Opaque and cutout layers (Rust `opaque`): dithered discard. The translucent
    // layer (ALPHA_CUTOUT 0.1, Rust `translucent`) multiplies its alpha instead.
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
