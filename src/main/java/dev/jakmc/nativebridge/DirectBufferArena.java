package dev.jakmc.nativebridge;
import java.nio.*;
import java.util.*;
/** Reuse within a bounded native-memory budget. Oversize work falls back without truncation. */
public final class DirectBufferArena {
 private static final ByteBuffer EMPTY=NativeJak.buffer(0);
 private final int budget;private final ArrayList<ByteBuffer> slots=new ArrayList<>();private int cursor,retained;
 public DirectBufferArena(int budget){if(budget<0)throw new IllegalArgumentException("Negative memory budget");this.budget=budget;}
 public void reset(){cursor=0;}
 public int retainedBytes(){return retained;}
 public ByteBuffer acquire(int bytes){
  if(bytes<0)throw new IllegalArgumentException("Negative buffer size");if(bytes==0)return EMPTY;
  ByteBuffer slot=cursor<slots.size()?slots.get(cursor):null;
  if(slot==null||slot.capacity()<bytes){int capacity=bytes<=1<<29?Math.max(256,Integer.highestOneBit(bytes-1)<<1):bytes;int old=slot==null?0:slot.capacity();
   if(capacity>budget||retained-old>budget-capacity)return NativeJak.buffer(bytes);
   slot=NativeJak.buffer(capacity);if(cursor==slots.size())slots.add(slot);else slots.set(cursor,slot);retained+=capacity-old;
  }
  cursor++;return slot.slice(0,bytes).order(ByteOrder.LITTLE_ENDIAN);
 }
}
