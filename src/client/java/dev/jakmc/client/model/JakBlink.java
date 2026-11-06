package dev.jakmc.client.model;

/**
 * Jak's and Daxter's blinking, OpenGOAL eye.gc (jak 1, 2 and 3 alike): at 60 frames a second a
 * countdown of 60 to 240 frames, the last ten of it reading blink-table (0, .667, .9, 1, 1, 1, 1,
 * .333, .1, 0) from its end; their clips carry no eye data (lid -1), so the lid is the table's value.
 * stand-in: the lid is drawn shut or open (the eyelid texture is whole), shut at .5 and up.
 */
final class JakBlink {
 private static final float[] TABLE={0,.667f,.9f,1,1,1,1,.333f,.1f,0};
 private static final java.util.Map<Integer,JakBlink> EYES=new java.util.HashMap<>();
 private long key,start,end;
 private JakBlink(int seed){key=Integer.toUnsignedLong(seed)*0x9e37_79b9_7f4a_7c15L;advance();}
 private void advance(){
  key^=key>>>29;key*=0xbf58_476d_1ce4_e5b9L;
  // eye.gc random-time: the next countdown of 60..240 frames, its last 10 the blink.
  end=end+60+(key>>>33)%181;start=end-10;
 }
 private boolean closed(float seconds){
  long frame=(long)(seconds*60);
  while(frame>=end)advance();
  if(frame<start)return false;
  return TABLE[9-(int)(frame-start)]>=.5f;
 }
 /** Whether the eyes of `seed` (an entity's own, Daxter's apart) are shut at `seconds` of its age. */
 static boolean shut(int seed,float seconds){
  if(EYES.size()>256)EYES.clear();
  return EYES.computeIfAbsent(seed,JakBlink::new).closed(seconds);
 }
}
