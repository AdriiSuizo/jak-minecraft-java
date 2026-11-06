#ifndef JAKMC_CAMERA_MASK_GLSL
#define JAKMC_CAMERA_MASK_GLSL

#include <minecraft:globals.glsl>

// Jak's third-person camera masks (dev.jakmc.client.camera.JakCameraCutout). Not a Jak port: Adrii's
// camera after "Recreating Baldur's Gate 3's Camera System in UE5" (occlusion masking, environment
// masks, fading rooms, fading dynamic objects). p is a surface point relative to the camera, in blocks.
// Each function returns how much of the surface is cut away: 0 keeps it, 1 removes it.

// Occlusion mask: a feathered ellipse around Jak on screen (measured at his depth, so it keeps his size),
// only on surfaces nearer to the camera than depthLimit.
float jak_mask_cut(vec3 p, float amount, float depthLimit) {
    if (amount <= 0.0) return 0.0;
    vec3 f = JakMask1.xyz;
    float z = dot(p, f);
    if (z <= 0.01) return 0.0;
    float front = 1.0 - smoothstep(depthLimit - 0.5, depthLimit, z);
    if (front <= 0.0) return 0.0;
    vec3 up = JakMask2.xyz;
    vec3 right = cross(f, up);
    vec3 c = JakMask0.xyz;
    float zj = max(dot(c, f), 0.05);
    vec2 at = vec2(dot(p, right), dot(p, up)) * (zj / z);
    vec2 d = (at - vec2(dot(c, right), dot(c, up))) / JakMask3.xy;
    return amount * front * (1.0 - smoothstep(0.55, 1.0, length(d)));
}

uint jak_room_word(int slot, bool rim, int word) {
    int v = word >> 2;
    int l = word & 3;
    if (slot == 0) return rim ? JakRoomARim[v][l] : JakRoomAFoot[v][l];
    return rim ? JakRoomBRim[v][l] : JakRoomBFoot[v][l];
}

// One room: the floor plan from the ceiling up, the wall ring above the ceiling, and the ring's walls on
// the camera's side (nearer than Jak) down to one block over the floor. q is nudged into its own block.
float jak_room_slot(vec3 q, int slot, vec4 room, vec4 info) {
    if (room.w <= 0.0) return 0.0;
    vec2 local = q.xz - room.xz;
    ivec2 col = ivec2(floor(local));
    if (col.x < 0 || col.y < 0 || col.x >= 32 || col.y >= 32) return 0.0;
    int index = col.y * 32 + col.x;
    uint bit = 1u << uint(index & 31);
    if ((jak_room_word(slot, false, index >> 5) & bit) != 0u) {
        return q.y >= info.x ? room.w : 0.0;
    }
    if ((jak_room_word(slot, true, index >> 5) & bit) == 0u) return 0.0;
    if (q.y >= info.x + 1.0) return room.w;
    if (q.y < info.y + 1.0) return 0.0;
    vec2 centre = room.xz + vec2(col) + 0.5;
    float z = dot(vec3(centre.x, q.y, centre.y), JakMask1.xyz);
    return room.w * (1.0 - smoothstep(info.z - 1.5, info.z, z));
}

float jak_room_cut(vec3 p) {
    if (JakRoomA.w <= 0.0 && JakRoomB.w <= 0.0) return 0.0;
    // Nudged away from the camera: a visible face belongs to the block behind it.
    vec3 q = p + normalize(p) * 0.02;
    return min(1.0, jak_room_slot(q, 0, JakRoomA, JakRoomAInfo) + jak_room_slot(q, 1, JakRoomB, JakRoomBInfo));
}

// 4x4 ordered dither: cuts partly faded opaque surfaces without sorting.
bool jak_fade_keep(float opacity, vec2 pixel) {
    if (opacity >= 0.999) return true;
    if (opacity <= 0.001) return false;
    int x = int(pixel.x) & 3;
    int y = int(pixel.y) & 3;
    const float bayer[16] = float[16](0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0);
    return opacity > (bayer[y * 4 + x] + 0.5) / 16.0;
}

#endif
