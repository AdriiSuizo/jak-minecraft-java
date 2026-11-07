#version 330
#extension GL_ARB_separate_shader_objects : require

#if defined(PER_FACE_LIGHTING) || !defined(NO_CARDINAL_LIGHTING)
#include <minecraft:light.glsl>
#endif
#include <minecraft:fog.glsl>
#include <minecraft:dynamictransforms.glsl>
#include <minecraft:projection.glsl>
#include <minecraft:sample_lightmap.glsl>
#include <jakmc:emerc.glsl>
#include <minecraft:globals.glsl>

layout(location = 0) in vec3 Position;
layout(location = 1) in vec4 Color;
layout(location = 2) in vec2 UV0;
layout(location = 3) in ivec2 UV1;
layout(location = 4) in ivec2 UV2;
layout(location = 5) in vec3 Normal;

#if !defined(NO_OVERLAY) && !defined(OIT_ALPHA_ONLY)
uniform sampler2D Sampler1;
#endif

#if !defined(EMISSIVE) && !defined(OIT_ALPHA_ONLY)
uniform sampler2D Sampler2;
#endif

layout(location = 0) out float sphericalVertexDistance;
layout(location = 1) out float cylindricalVertexDistance;

#ifdef PER_FACE_LIGHTING
layout(location = 2) out vec4 vertexPerFaceColorBack;
layout(location = 3) out vec4 vertexPerFaceColorFront;
#else
layout(location = 2) out vec4 vertexColor;
#endif

#ifndef EMISSIVE
layout(location = 4) out vec4 lightMapColor;
#endif

#ifndef NO_OVERLAY
layout(location = 5) out vec4 overlayColor;
#endif

layout(location = 6) out vec2 texCoord0;
#ifdef GLINT
layout(location = 7) out vec2 texCoordGlint;
#endif
// Jak's camera fading of objects (jakmc:camera_mask.glsl): the camera-relative point, and 1 in a
// perspective (world) view, 0 in the GUI's orthographic one.
layout(location = 8) out vec3 jakSurface;
layout(location = 9) out float jakWorld;
#ifdef JAK_ENVMAP
// merc's environment pass in the same draw: the sphere map of the reflected view (GltfModel's env pass, per vertex).
layout(location = 10) out vec2 jakEnvCoord;
#endif

void main() {
    gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);
    jakSurface = Position;
    jakWorld = ProjMat[3][3] < 0.5 ? 1.0 : 0.0;

    sphericalVertexDistance = fog_spherical_distance(Position);
    cylindricalVertexDistance = fog_cylindrical_distance(Position);

    float jakLength = length(Normal);
    bool jakRig = jakLength > 0.1 && jakLength < 0.75;
    vec3 jakNormal = jakRig ? Normal / jakLength : Normal;
    jakRig = jakRig && jakWorld > 0.5;
#ifdef PER_FACE_LIGHTING
    vec2 light = minecraft_compute_light(Light0_Direction, Light1_Direction, jakNormal);
    vertexPerFaceColorBack = minecraft_mix_light_separate(-light, Color);
    vertexPerFaceColorFront = minecraft_mix_light_separate(light, Color);
#elif defined(NO_CARDINAL_LIGHTING)
    vertexColor = Color;
#else
    vertexColor = minecraft_mix_light(Light0_Direction, Light1_Direction, jakNormal, Color);
#endif

#if !defined(EMISSIVE) && !defined(OIT_ALPHA_ONLY)
    lightMapColor = sample_lightmap(Sampler2, UV2);
    if (jakRig) {
        // Rust minecraft_light.rs mc_entity_light: lights.gc's three directional slots and 0.3 ambient, with 0.2 fill,
        // by the sky's own part of the light; the blocks' added on; up to 1.5.
        vec3 jakDark = sample_lightmap(Sampler2, ivec2(0, 0)).rgb;
        vec3 jakSky = max(sample_lightmap(Sampler2, ivec2(0, UV2.y)).rgb - jakDark, vec3(0.0));
        vec3 jakLocal = sample_lightmap(Sampler2, ivec2(UV2.x, 0)).rgb;
        vec3 jakKey = normalize(JakLightSun.xyz);
        vec3 jakFill = normalize(vec3(-jakKey.x, abs(jakKey.y), -jakKey.z));
        // stand-in: the Rust's bounce takes the fog's colour.
        vec3 jakRigLight = vec3(0.3) + JakLightSun.w * JakLightKey.rgb * max(dot(jakNormal, jakKey), 0.0)
            + vec3(0.2 * max(dot(jakNormal, jakFill), 0.0)) + vec3(0.1 * max(-jakNormal.y, 0.0));
        vec3 jakLit = clamp(jakSky * jakRigLight + jakLocal, vec3(0.0), vec3(1.5));
        // stand-in (Adrii 07/11, seen in a cave): update-mood-interior's 0.333 and overhead 0.667, neutral, only out of the sky.
        float jakIndoors = 1.0 - smoothstep(4.0, 10.0, float(UV2.y) / 16.0);
        jakLit = max(jakLit, vec3(0.333 + 0.667 * max(jakNormal.y, 0.0)) * jakIndoors);
        // powerups.gc: color-mult scales the lights, color-emissive adds to them.
        lightMapColor = vec4(jakLit * JakLightMult.rgb + JakLightGlow.rgb, 1.0);
    }
#endif
#ifdef PER_FACE_LIGHTING
    if (jakRig) {
        vertexPerFaceColorBack = Color;
        vertexPerFaceColorFront = Color;
    }
#else
    if (jakRig) vertexColor = Color;
#endif

#if !defined(NO_OVERLAY) && !defined(OIT_ALPHA_ONLY)
    overlayColor = texelFetch(Sampler1, UV1, 0);
#endif

    texCoord0 = UV0;

#ifdef JAK_ENVMAP
    jakEnvCoord = jak_emerc_st(ModelViewMat, Position, Normal);
#endif

#ifdef APPLY_TEXTURE_MATRIX
    texCoord0 = (TextureMat * vec4(UV0, 0.0, 1.0)).xy;
#endif

#ifdef GLINT
    texCoordGlint = (TextureMat * vec4(UV0, 0.0, 1.0)).xy;
#endif
}
