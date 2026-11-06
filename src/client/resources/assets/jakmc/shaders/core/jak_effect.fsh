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
#ifdef JAK_GLOW
// sprite-glow.gc: the glow is soft. Its texture is blurred over a 5x5 grid of taps 0.1 apart: JakEffects widens
// the glow's quad by the 0.2 margin this reaches. Taps off the texture add nothing, so the glow fades at the edge.
vec4 glowBlur(){
 vec3 sum=vec3(0.0);
 for(int i=-2;i<=2;i++)for(int j=-2;j<=2;j++){
  vec2 at=texCoord0+vec2(float(i),float(j))*0.1;
  float inside=step(0.0,at.x)*step(at.x,1.0)*step(0.0,at.y)*step(at.y,1.0);
  sum+=texture(Sampler0,at).rgb*exp(-float(i*i+j*j)*0.25)*inside;
 }
 return vec4(sum/10.846223,1.0);
}
#endif
void main(){
#ifdef JAK_GLOW
#ifdef OIT_ALPHA_ONLY
 vec4 tex=vec4(1.0); // the alpha-only passes never read colour
#else
 vec4 tex=glowBlur();
#endif
#else
 vec4 tex=texture(Sampler0,texCoord0);
#endif
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
 #ifdef JAK_GUI
 // Item render targets are transparent; PS2 additive alpha=1 creates black squares there.
 vec3 emission=tex.rgb*vertexColor.rgb*2.0;
 #if JAK_BLEND == 3
 emission*=vertexColor.a*2.0;
 #else
 emission*=clamp(tex.a*vertexColor.a*4.0,0.0,1.0);
 #endif
 float coverage=clamp(max(emission.r,max(emission.g,emission.b)),0.0,1.0);
 color=vec4(emission/max(coverage,0.001),coverage);
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
