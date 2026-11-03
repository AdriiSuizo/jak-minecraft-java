import dev.jakmc.nativebridge.DirectBufferArena;
import java.nio.*;
/** JNI capacity, alias and bounded retention contract, independent of Minecraft. */
public class DirectBufferArenaCheck {
 public static void main(String[] args){var arena=new DirectBufferArena(1024);var a=arena.acquire(40);a.putInt(0,123);var b=arena.acquire(80);b.putInt(0,456);if(!a.isDirect()||a.capacity()!=40||a.order()!=ByteOrder.LITTLE_ENDIAN||a.getInt(0)!=123||b.capacity()!=80)throw new AssertionError("Capacity/order/alias");var large=arena.acquire(8192);large.putInt(0,789);if(large.capacity()!=8192||arena.retainedBytes()>1024||a.getInt(0)!=123)throw new AssertionError("Uncached fallback");arena.reset();var again=arena.acquire(24);if(again.capacity()!=24||again.getInt(0)!=123)throw new AssertionError("Reset reuse");if(arena.acquire(0).capacity()!=0)throw new AssertionError("Empty");System.out.println("PASS exact direct capacity, independent live slices, reset reuse, bounded retention, oversize work, zero buffer");}
}
