#version 330
#extension GL_ARB_separate_shader_objects : require
#include <minecraft:dynamictransforms.glsl>
// OpenGOAL BlitDisplays::do_zoom_blur's fullscreen draws (ISC, third_party/opengoal): the picture's
// rectangle (xmin, ymin)..(xmax, ymax), top down as on the PS2's screen, over the whole screen at an alpha.
// TextureMat[0]: xmin, ymin, xmax, ymax; TextureMat[1].x: the alpha.
uniform sampler2D InSampler;
layout(location=0) in vec2 texCoord;
layout(location=0) out vec4 fragColor;
void main(){
 vec4 rect=TextureMat[0];
 float x=mix(rect.x,rect.z,texCoord.x);
 float y=1.0-mix(rect.y,rect.w,1.0-texCoord.y);
 fragColor=vec4(texture(InSampler,vec2(x,y)).rgb,TextureMat[1].x);
}
