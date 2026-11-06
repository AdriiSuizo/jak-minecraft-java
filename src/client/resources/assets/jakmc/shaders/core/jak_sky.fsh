#version 330
#extension GL_ARB_separate_shader_objects : require
#include <minecraft:dynamictransforms.glsl>
// Jak 1's sky (sky-tng.gc), as render_gpu drawsurf/minecraft_world.rs draws it. jak_sky.rs shader_data rides in
// TextureMat's columns ([1] sky textures and weights, [3] cloud offsets, [4] overcast, [5] desert sky) and
// ColorModulator ([6] the fog colour, [2].y the lightning's weight; its texture is snapshot 2).
uniform sampler2D Sampler0;
layout(location=0) in vec3 ray;
layout(location=0) out vec4 fragColor;
// jak_sky_texel: village1's eight sky textures, 32 x 32 from x 288.
vec3 jak_sky_texel(float index,vec2 st){
 vec2 lo=vec2(288.0+index*32.0+0.5,0.5);
 vec2 p=clamp(vec2(288.0+index*32.0,0.0)+st*32.0,lo,lo+vec2(31.0));
 return textureLod(Sampler0,p/vec2(1024.0,128.0),0.0).rgb;
}
// jak_cloud_texel: vil1-clouds, 64 x 64 at x 544, repeating.
vec4 jak_cloud_texel(vec2 st){
 vec2 p=vec2(544.0,0.0)+clamp(fract(st)*64.0,vec2(0.5),vec2(63.5));
 return textureLod(Sampler0,p/vec2(1024.0,128.0),0.0);
}
// jak_roof_st: the pyramid roof from the horizon's corners 10 km out to its top 2.5 km up.
vec2 jak_roof_st(vec3 j){
 float ax=abs(j.x),az=abs(j.z);
 float t=1.0/max(ax+az+4.0*max(j.y,0.0),1e-6);
 float wx=t*ax,wz=t*az,wc=1.0-wx-wz;
 vec2 cx=j.x>=0.0?vec2(1.0,0.0):vec2(0.0,1.0);
 vec2 cz=j.z>=0.0?vec2(1.0,1.0):vec2(0.0,0.0);
 return cx*wx+cz*wz+vec2(0.5)*wc;
}
// jak_cloud_layer (sky-tng-setup-cloud-layer): a square funnel, the texture nine times across, added on.
vec3 jak_cloud_layer(vec3 j,vec2 offset,float twist,vec3 colour){
 if(j.y<=0.0)return vec3(0.0);
 float rd=max(abs(j.x),abs(j.z));
 float t=22.0/(20.0*j.y+rd);
 if(t*rd<2.0)t=1.0/j.y;
 vec3 hit=j*t;
 float r=max(abs(hit.x),abs(hit.z));
 float angle=twist*clamp((22.0-r)/20.0,0.0,1.0);
 float c=cos(angle),s=sin(angle);
 vec2 local=vec2(hit.x*c-hit.z*s,hit.x*s+hit.z*c);
 vec2 st=local*(9.0/44.0)+vec2(0.5)+offset;
 float alpha=clamp((22.0-r)/10.0,0.0,1.0);
 vec4 texel=jak_cloud_texel(st);
 return texel.rgb*colour/128.0*texel.a*alpha;
}
// WGSL's smoothstep(0.0, -0.08, x) (GLSL leaves a falling edge undefined).
float below(float x){float t=clamp(x/-0.08,0.0,1.0);return t*t*(3.0-2.0*t);}
void main(){
 vec3 r=normalize(ray);
 // Jak's axes: his sun rises where Minecraft's does (+x).
 vec3 j=vec3(-r.z,r.y,r.x);
 vec3 horizon=vec3(j.x,max(j.y,0.0),j.z);
 vec2 st=jak_roof_st(horizon);
 vec4 skies=TextureMat[0];
 vec3 colour=jak_sky_texel(skies.x,st)*skies.y+jak_sky_texel(skies.z,st)*skies.w;
 colour+=jak_cloud_layer(j,TextureMat[1].xy,0.0,vec3(20.0,20.0,20.0));
 colour+=jak_cloud_layer(j,TextureMat[1].zw,asin(0.05584),vec3(20.0,20.0,30.0));
 float height=clamp(j.y,0.0,1.0);
 vec3 desert=mix(ColorModulator.rgb,TextureMat[3].rgb,sqrt(height));
 colour=mix(colour,desert,TextureMat[3].a);
 colour=mix(colour,TextureMat[2].rgb,TextureMat[2].a);
 colour+=jak_sky_texel(2.0,st)*ColorModulator.a;
 colour=mix(colour,ColorModulator.rgb,below(j.y));
 fragColor=vec4(min(colour,vec3(1.0)),1.0);
}
