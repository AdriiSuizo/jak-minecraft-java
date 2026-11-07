#ifndef MINECRAFT_GLOBALS_GLSL
#define MINECRAFT_GLOBALS_GLSL

// Vanilla 26.3 block plus Jak's third-person camera masks (dev.jakmc.client.camera.JakCameraCutout),
// camera-relative blocks:
// JakMask0 = Jak's body centre, terrain mask amount; JakMask1 = camera forward, Jak's feet height;
// JakMask2 = camera up, entity mask amount; JakMask3 = ellipse half width/height, terrain/entity depth limits.
// JakCap = 1 when Jak is under cover, fill's reach round Jak, its feather, Jak's depth (terrain.fsh JAK_CAP).
// JakRoom{A,B} = room grid origin, fade; JakRoom{A,B}Info = cut height, floor height, near-wall depth limit;
// JakRoom{A,B}Foot/Rim = 32x32 column bits of the floor plan and of the wall ring round it.
layout(std140) uniform Globals {
    ivec3 CameraBlockPos;
    float GlintAlpha;
    vec3 CameraOffset;
    float GameTime;
    vec2 ScreenSize;
    int MenuBlurRadius;
    int UseRgss;
    vec4 JakMask0;
    vec4 JakMask1;
    vec4 JakMask2;
    vec4 JakMask3;
    vec4 JakCap;
    vec4 JakRoomA;
    vec4 JakRoomAInfo;
    uvec4 JakRoomAFoot[8];
    uvec4 JakRoomARim[8];
    vec4 JakRoomB;
    vec4 JakRoomBInfo;
    uvec4 JakRoomBFoot[8];
    uvec4 JakRoomBRim[8];
};

#endif
