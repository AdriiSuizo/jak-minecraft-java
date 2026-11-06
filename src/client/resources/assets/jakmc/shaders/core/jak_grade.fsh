#version 330
#extension GL_ARB_separate_shader_objects : require
#include <minecraft:dynamictransforms.glsl>
// render_gpu drawsurf/enhanced_grade.rs, Jak Enhanced Trilogy's area grading (ISC, third_party/jak-enhanced-trilogy).
// TextureMat's columns: lift (rgb, saturation), gain (rgb, contrast curve), params (warmth, strength, -, on).
// Minecraft's main target already holds the display-referred picture the original grades: no sRGB crossing here.
uniform sampler2D InSampler;
layout(location=0) in vec2 texCoord;
layout(location=0) out vec4 fragColor;
void main(){
 vec4 lift=TextureMat[0],gain=TextureMat[1],params=TextureMat[2];
 vec4 original=texture(InSampler,texCoord);
 vec3 c=original.rgb;
 c=c*gain.xyz+lift.xyz*(1.0-clamp(c,vec3(0.0),vec3(1.0)));
 float l=dot(c,vec3(0.2126,0.7152,0.0722));
 c=max(mix(vec3(l),c,lift.w),vec3(0.0));
 // Clamp the curve input, not the light, to avoid negative highlights.
 vec3 bounded=clamp(c,vec3(0.0),vec3(1.0));
 c=mix(c,bounded*bounded*(3.0-2.0*bounded),gain.w);
 c+=params.x*vec3(1.0,0.25,-0.75)*(l-0.35)*(1.0-0.6*smoothstep(0.55,0.9,l));
 fragColor=vec4(mix(original.rgb,max(c,vec3(0.0)),params.y),original.a);
}
