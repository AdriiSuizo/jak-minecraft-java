#version 330
#extension GL_ARB_separate_shader_objects : require
#include <minecraft:dynamictransforms.glsl>
// The sky's bodies as Jak's sprites (render/extract.rs push_jak_sparks): colour doubled, alpha doubled twice.
uniform sampler2D Sampler0;
layout(location=0) in vec2 texCoord0;
layout(location=1) in vec4 vertexColor;
layout(location=0) out vec4 fragColor;
void main(){
 vec4 tex=texture(Sampler0,texCoord0);
 vec4 color=vec4(tex.rgb*vertexColor.rgb*2.0,tex.a*vertexColor.a*4.0)*ColorModulator;
 if(color.a<0.001)discard;
 fragColor=color;
}
