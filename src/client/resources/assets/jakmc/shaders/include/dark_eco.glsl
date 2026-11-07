#ifndef JAKMC_DARK_ECO_GLSL
#define JAKMC_DARK_ECO_GLSL

// Dark eco (Jak 1's dark-eco-pool, the final silo's: look 41). Its faces come tinted pure magenta (DarkEcoClient), which
// nothing else in the terrain is. Rust render_gpu drawsurf/minecraft_world.rs darkeco_wave and darkeco_fragment.

bool jak_dark_eco_face(vec4 tint) {
    return tint.g < 0.004 && tint.r > 0.2 && abs(tint.r - tint.b) < 0.02;
}

// ripple-for-finalboss-dark-eco-pool: each wave's period 16 / |div| cells, its offset running at its speed.
float jak_dark_eco_one(vec2 p, float scale, vec2 div, float speed) {
    const float cell = 6.2831853 / 16.0;
    return scale * cos(dot(p, div) * cell - JakEcoClock.x * speed * cell * length(div));
}

// In blocks: 0.75 m at 128.
float jak_dark_eco_wave(vec2 p) {
    float h = jak_dark_eco_one(p, 25.0, vec2(2.0, 0.0), -2.0)
            + jak_dark_eco_one(p, 15.0, vec2(-2.0, 2.0), 3.0)
            + jak_dark_eco_one(p, 4.0, vec2(5.0, 3.0), 4.0);
    return clamp(h, -127.0, 127.0) / 128.0 * 0.5208;
}

#ifdef JAK_DARK_ECO_FRAGMENT
// A texel of a sprite in the block atlas (x, y, width, height in pixels), between its pixels, wrapping as a sheet does.
vec4 jak_dark_eco_sheet(sampler2D atlas, vec4 sprite, vec2 st) {
    vec2 p = st * sprite.zw - 0.5;
    vec2 low = floor(p), f = p - low;
    vec2 high = mod(low + 1.0, sprite.zw);
    low = mod(low, sprite.zw);
    ivec2 at = ivec2(sprite.xy);
    vec4 a = texelFetch(atlas, at + ivec2(low), 0);
    vec4 b = texelFetch(atlas, at + ivec2(high.x, low.y), 0);
    vec4 c = texelFetch(atlas, at + ivec2(low.x, high.y), 0);
    vec4 d = texelFetch(atlas, at + ivec2(high), 0);
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

// The pool's texture is black: all its look is the environment map, the view reflected off the waves, laid on as merc's
// environment pass lays it (the map twice over, by the mottled strength the texture's alpha leaves). No flow: a pool lies
// still, so a poured stream shines as it does.
vec4 jak_dark_eco(sampler2D atlas, vec3 world, vec3 eye, float light) {
    float e = 0.5;
    float dx = jak_dark_eco_wave(world.xz + vec2(e, 0.0)) - jak_dark_eco_wave(world.xz - vec2(e, 0.0));
    float dz = jak_dark_eco_wave(world.xz + vec2(0.0, e)) - jak_dark_eco_wave(world.xz - vec2(0.0, e));
    vec3 face = normalize(cross(dFdx(world), dFdy(world)));
    if (dot(face, eye) > 0.0) {
        face = -face;
    }
    bool top = face.y > 0.5;
    vec3 normal = top ? normalize(vec3(-dx * JakEcoClock.y, 2.0 * e, -dz * JakEcoClock.y)) : normalize(face + vec3(-dx, 0.3, -dz));
    // A sheet every four blocks, drifting slowly.
    vec2 sheet = top ? world.xz : vec2(world.x + world.z, world.y);
    vec4 texel = jak_dark_eco_sheet(atlas, JakEcoWater, fract(sheet / 4.0 + JakEcoClock.x * vec2(0.01, 0.006)));
    vec3 r = reflect(normalize(eye), normal);
    float m = 2.0 * sqrt(r.x * r.x + r.z * r.z + (r.y + 1.0) * (r.y + 1.0));
    vec3 shine = jak_dark_eco_sheet(atlas, JakEcoShine, vec2(r.x / m + 0.5, r.z / m + 0.5)).rgb;
    // stand-in: the map squared, for what the PS2's mood and vertex colours leave of it: black but for its swirls.
    shine *= shine * 1.2;
    return vec4(shine * 2.0 * texel.a * clamp(light + 0.35, 0.35, 1.0), 0.94);
}

#endif

#endif
