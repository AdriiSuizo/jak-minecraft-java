#version 330
#extension GL_ARB_separate_shader_objects : require

#include <minecraft:fog.glsl>
#include <minecraft:dynamictransforms.glsl>
#include <minecraft:oit.glsl>

// merc's environment pass (emerc): the map's colour twice over, by the base's alpha (128 one), added on
// (Rust reference minecraft_world.rs: base + shine.rgb * 2 * texel.a). Unlit, as the original's.
uniform sampler2D Sampler0; // the environment map
uniform sampler2D Sampler1; // the base colour: its alpha is the strength

layout(location = 0) in float sphericalVertexDistance;
layout(location = 1) in float cylindricalVertexDistance;
layout(location = 6) in vec2 texCoord0;
layout(location = 7) in vec2 texCoord1;
layout(location = 8) in float shineAmount;

#ifndef OIT_ALPHA_ONLY
layout(location = 0) out vec4 fragColor;
#endif

void main() {
    vec3 shine = texture(Sampler0, texCoord0).rgb * texture(Sampler1, texCoord1).a * 2.0 * shineAmount;
    if (max(shine.r, max(shine.g, shine.b)) < 0.001) {
        discard;
    }
    float visibility = 1.0 - total_fog_value(sphericalVertexDistance, cylindricalVertexDistance, FogEnvironmentalStart, FogEnvironmentalEnd, FogRenderDistanceStart, FogRenderDistanceEnd);
    vec4 color = vec4(shine * visibility, 1.0);

#ifdef OIT_ALPHA_ONLY
    executeAlphaOnlyPhase(gl_FragCoord.z, clamp(color.a, 0.0, 0.99));
#else
#ifdef OIT_ACCUMULATE
    color = sampleColorForAccumulation(color);
#endif
    fragColor = color;
#endif
}
