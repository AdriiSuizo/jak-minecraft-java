#version 330
#extension GL_ARB_separate_shader_objects : require
#include <minecraft:dynamictransforms.glsl>
#include <minecraft:projection.glsl>
layout(location=0) in vec3 Position;
layout(location=0) out vec3 ray;
// A cube round the eye: its interpolated corner is the view ray (world axes).
void main(){
 gl_Position=ProjMat*ModelViewMat*vec4(Position,1.0);
 ray=Position;
}
