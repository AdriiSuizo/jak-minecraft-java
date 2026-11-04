package dev.jakmc.client.model;

/** Scoped to Minecraft's picture-in-picture entity pass, never world extraction. */
public final class JakGuiContext {
    private static final ThreadLocal<Integer> DEPTH=ThreadLocal.withInitial(()->0);
    public static boolean active(){return DEPTH.get()>0;}
    public static void enter(){DEPTH.set(DEPTH.get()+1);}
    public static void leave(){DEPTH.set(Math.max(0,DEPTH.get()-1));}
    private JakGuiContext(){}
}
