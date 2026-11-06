#version 330
#extension GL_ARB_separate_shader_objects : require
#include <minecraft:dynamictransforms.glsl>
// OpenGOAL slow_time.vert/frag (ISC, third_party/opengoal), drawn over the last picture shown with
// alpha blending; blit-displays.gc's colour filter (1, 1, 1.5) on the new picture first.
// TextureMat[0]: amount (the corners' alpha), the filter's blue.
uniform sampler2D InSampler;
layout(location=0) in vec2 texCoord;
layout(location=0) out vec4 fragColor;
void main(){
 float amount=TextureMat[0].x,blue=TextureMat[0].y;
 // The fan from the centre (0.75, 0.75, 0.75, 1.5) to the corners (1, 1, 1, amount).
 vec2 ndc=texCoord*2.0-1.0;
 float centre=1.0-max(abs(ndc.x),abs(ndc.y));
 vec4 color=mix(vec4(1.0,1.0,1.0,amount),vec4(0.75,0.75,0.75,1.5),centre);
 vec3 c=texture(InSampler,texCoord).rgb*vec3(1.0,1.0,blue);
 fragColor=clamp(vec4(c*color.rgb,color.a),0.0,1.0);
}
