// emerc.vert's st, from the VU1 program: the map laid out in the camera's axes, x right, y down, z ahead (the PS2's).
// Minecraft has Position and Normal in the world's axes round the camera; ModelViewMat turns them.
vec2 jak_emerc_st(mat4 modelView, vec3 position, vec3 normal) {
    vec3 view = (modelView * vec4(position, 1.0)).xyz;
    vec3 turned = mat3(modelView) * (length(normal) > 0.0 ? normalize(normal) : vec3(0.0, 0.0, 1.0));
    vec3 p = vec3(view.x, -view.y, -view.z);
    vec3 n = vec3(turned.x, -turned.y, -turned.z - 1.0);
    n.z = min(n.z, -0.0001);
    vec3 v = p + n * (dot(p, n) / n.z);
    float size = length(v);
    return size > 0.0 ? 0.5 + 0.5 * v.xy / size : vec2(0.5);
}
