package dev.jakmc.nativebridge;

import java.nio.*;
import java.nio.file.*;
import java.io.*;
import java.util.*;

/** Original merc-ctrl segment table. Surfaces are split by material, not by
 * segment: map their triangle counts back to each authored merc effect. */
public record ModelSegments(int[] effects,long[] segments) {
 public boolean shown(int surface,long mask){long hidden=0;for(int i=0;i<segments.length;i++)if((mask&(1L<<i))==0)hidden|=segments[i];return (hidden&(1L<<effects[surface]))==0;}
 public static ModelSegments read(Path art,String lod,int[] triangles)throws IOException{
  byte[] bytes=Files.readAllBytes(art);ByteBuffer b=ByteBuffer.wrap(bytes).order(ByteOrder.LITTLE_ENDIAN);
  boolean modern=b.getShort(8)==5;int data=modern?b.getInt(4):16;
  if(data<0||data>=bytes.length)return null;byte[] needle=(lod+"\0").getBytes(java.nio.charset.StandardCharsets.US_ASCII);
  for(int at=data+4;at+needle.length<=bytes.length;at++){
   if(!Arrays.equals(bytes,at,at+needle.length,needle,0,needle.length))continue;
   int name=at-4-data;if(b.getInt(data+name)!=lod.length())continue;
   for(int ctrl=0;ctrl+data+4<bytes.length;ctrl+=4){
    if(b.getInt(data+ctrl)!=name)continue;
    int header=data+ctrl+(modern?40:24);if(header+128>bytes.length)continue;
    int count=b.getInt(header+24),shaders=Byte.toUnsignedInt(b.get(header+37));if(count<1||count>=64||shaders==0||header+128+count*32>bytes.length)continue;
    int table=b.getInt(data+ctrl+(modern?20:16));if(table<0||data+table+12>bytes.length)continue;
    int length=b.getInt(data+table);if(length<0||length>64||data+table+12+length*8>bytes.length)continue;
    long[] segments=new long[length];for(int i=0;i<length;i++)segments[i]=b.getLong(data+table+12+i*8);
    int[] effects=new int[triangles.length];int effect=0,taken=0;
    for(int s=0;s<triangles.length;s++){
     while(effect<count&&taken>=Short.toUnsignedInt(b.getShort(header+128+effect*32+22))){effect++;taken=0;}
     if(effect>=count)return null;taken+=triangles[s];effects[s]=effect;
    }
    if(effect+1==count&&taken==Short.toUnsignedInt(b.getShort(header+128+(count-1)*32+22)))return new ModelSegments(effects,segments);
   }
  }return null;
 }
}
