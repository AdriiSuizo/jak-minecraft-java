package dev.jakmc.client.input;
/** Original Rust Jak host input: right radial deadzone .18, 180 degrees/second.
 * Left stick stays raw; native Pad::update owns Jak's .3 movement deadzone. */
public final class ControllerMath {
 private ControllerMath(){}
 public static float stick(short value){return value<0?value/32768f:value/32767f;}
 public static float trigger(short value){return Math.clamp(value/32767f,0,1);}
 public static float[] look(float x,float y){
  float length=(float)Math.hypot(x,y);if(length<=.18f)return new float[]{0,0};
  float speed=(Math.min(length,1)-.18f)/.82f;
  return new float[]{x/length*speed,y/length*speed};
 }
}
