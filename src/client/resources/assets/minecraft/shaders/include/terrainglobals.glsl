#ifndef MINECRAFT_TERRAINGLOBLAS_GLSL
#define MINECRAFT_TERRAINGLOBALS_GLSL

// Vanilla 26.3 block plus the Jak camera cutaway (dev.jakmc.client.camera.JakCameraCutout):
// JakFadeEye = camera eye (camera-relative blocks) and opening 0..1; JakFadeTarget = pivot and feet height.
layout(std140) uniform TerrainUniform {
    mat4 ModelViewMat;
    ivec2 TextureSize;
    vec4 JakFadeEye;
    vec4 JakFadeTarget;
};

#endif
