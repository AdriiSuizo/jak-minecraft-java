#version 330
#extension GL_ARB_separate_shader_objects : require
#include <minecraft:fog.glsl>
#include <minecraft:dynamictransforms.glsl>
#include <minecraft:oit.glsl>
uniform sampler2D Sampler0;
layout(location=0) in float sphericalVertexDistance;
layout(location=1) in float cylindricalVertexDistance;
layout(location=2) in vec4 vertexColor;
layout(location=6) in vec2 texCoord0;
#ifndef OIT_ALPHA_ONLY
layout(location=0) out vec4 fragColor;
#endif
void main(){
 vec4 tex=texture(Sampler0,texCoord0);
 vec4 color=vec4(tex.rgb*vertexColor.rgb*2.0,tex.a*vertexColor.a*4.0)*ColorModulator;
 float visibility=1.0-total_fog_value(sphericalVertexDistance,cylindricalVertexDistance,FogEnvironmentalStart,FogEnvironmentalEnd,FogRenderDistanceStart,FogRenderDistanceEnd);
 #if JAK_BLEND == 3
 color.rgb*=vertexColor.a*2.0*visibility;
 color.a=1.0;
 #elif JAK_BLEND == 0
 color=apply_fog(color,sphericalVertexDistance,cylindricalVertexDistance,FogEnvironmentalStart,FogEnvironmentalEnd,FogRenderDistanceStart,FogRenderDistanceEnd,FogColor);
 #else
 color.a*=visibility;
 #endif
 if(color.a<0.001)discard;
 #ifdef OIT_ALPHA_ONLY
 executeAlphaOnlyPhase(gl_FragCoord.z,clamp(color.a,0.0,0.99));
 #else
 #ifdef OIT_ACCUMULATE
 #if JAK_BLEND == 1
 color.rgb=-color.rgb;
 #endif
 color=sampleColorForAccumulation(color);
 #endif
 fragColor=color;
 #endif
}
