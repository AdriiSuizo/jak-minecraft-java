package dev.jakmc.client.model;

import com.mojang.blaze3d.vertex.PoseStack;
import dev.jakmc.JakData;
import dev.jakmc.RuntimeConfig;
import dev.jakmc.content.JakIrisDoor;
import java.io.IOException;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.blockentity.BlockEntityRenderer;
import net.minecraft.client.renderer.blockentity.state.BlockEntityRenderState;
import net.minecraft.client.renderer.feature.ModelFeatureRenderer;
import net.minecraft.client.renderer.state.level.CameraRenderState;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
import net.minecraft.world.phys.Vec3;
import org.joml.Matrix4f;
import org.joml.Vector3f;

/**
 * The 3x3 doors, fitted to their blocks. The Geyser Rock iris door (jng-iris-door): its blades turn open as
 * jng-iris-door-idle plays forward (eco-door door-opening, speed 1), back as it closes. stand-in (Adrii 06/11):
 * door-open hides it all; here its frame (the "base" joint's ring) stays, the blades gone, and the ring's
 * outside is drawn square to sit with the blocks round it. Haven's airlock: com-airlock-outer and
 * com-airlock-inner back to back, each its "idle" clip run as airlock.gc's open and close states run it, with their sounds.
 */
public final class IrisDoorRenderer implements BlockEntityRenderer<JakIrisDoor.Door,IrisDoorRenderer.State> {
    /** One door's mesh, its clip, its fit to three blocks and how airlock.gc runs it: the open state's
     * stretches (to an artist frame, at a speed) and the close state's, the last of each to the clip's end. */
    private record Leaf(GltfModel model,String clip,Vector3f middle,float fit,boolean flat,float depth,int cells,float thick,
                        float[][] opening,float[][] closing,String[] openSounds,String[] closeSounds,float gain){
        float frames(){return Math.max(model.duration(clip),.01f)*60;}
    }
    private static final float MOST_SECONDS=2.5f;
    private static Leaf iris;private static Leaf[] airlock;private static boolean irisMissing,airlockMissing;
    /** The iris door open: its frame alone. */
    private static GltfModel.Posed irisFrame;
    private static final String[] BLADES={"rote1","rote2","rote3","rote4","rote5","rote6","rote7","rote8"};
    /** Adrii 06/11, the airlock too loud: airlock.gc's CTYWIDE1 streams take Jak 2's x4.5 bank gain on import
     * (import_audio.py mix_gain) yet stand some 10 dB over its bank sounds (mean -8 to -15 dB against a median
     * -25); played at 0.3 they sit with them. */
    private static final float AIRLOCK_GAIN=.3f;
    public static final class State extends BlockEntityRenderState {
        GltfModel.Posed[] poses;Leaf[] leaves;Direction facing;
    }
    @Override public State createRenderState(){return new State();}
    @Override public int getViewDistance(){return 64;}

    private static Leaf leaf(java.nio.file.Path file,String name,String clip,int cells,float[][] opening,float[][] closing,String[] openSounds,String[] closeSounds,float gain) throws IOException{
        var model=new GltfModel(file,name,true);
        // The iris door's frame squared (the ring's outer edge 4 m out, its inner 2.83 m, both from jng-iris-door-lod0).
        if(name.equals("jng-iris-door"))model.squareRing("base",3.5f);
        var closed=model.pose(clip,0);
        var bounds=model.bounds(closed,new Matrix4f());
        var size=new Vector3f(bounds[1]).sub(bounds[0]);
        var middle=new Vector3f(bounds[0]).add(bounds[1]).mul(.5f);
        boolean flat=size.y<Math.min(size.x,size.z)*.5f;
        float across=flat?Math.max(size.x,size.z):Math.max(size.x,size.y);
        float depth=flat?size.y:size.z;
        float fit=across>0?cells/across:1;
        // The airlock's two doors share one block of thickness (Adrii 06/11: thinner, same size).
        float thick=clip.startsWith("com-airlock")&&depth>0?Math.min(fit,.48f/depth):fit;
        return new Leaf(model,clip,middle,fit,flat,depth,cells,thick,opening,closing,openSounds,closeSounds,gain);
    }
    private static Leaf[] leaves(boolean isAirlock){
        if(isAirlock){
            if(airlock==null&&!airlockMissing)try{
                var root=RuntimeConfig.gameAssets("jak2").resolve("levels/ctyfarma");
                // com-airlock-outer: pre-open 35, lock 45, open 45, pre-open speed 2; its sounds by stretch.
                var outer=leaf(root.resolve("com-airlock-outer-lod0.glb"),"com-airlock-outer","com-airlock-outer-idle",3,
                    new float[][]{{45,2},{45,2},{-1,1}},new float[][]{{45,2},{0,1}},
                    new String[]{"jak2/airlock_slider",null,"jak2/airlock_seal","jak2/airlock_hit"},new String[]{"jak2/airlock_open","jak2/airlock_hit",null},AIRLOCK_GAIN);
                // com-airlock-inner: lock 37, pre-open 65, open 75, pre-open speed 0.9; its gear turning and unlocking.
                var inner=leaf(root.resolve("com-airlock-inner-lod0.glb"),"com-airlock-inner","com-airlock-inner-idle",3,
                    new float[][]{{37,.9f},{75,2},{-1,1}},new float[][]{{75,2},{0,1}},
                    new String[]{"jak2/airlock_gear","jak2/airlock_turn","jak2/airlock_unlock",null},new String[]{null,null,null},AIRLOCK_GAIN);
                airlock=new Leaf[]{outer,inner};
            }catch(IOException failure){airlockMissing=true;dev.jakmc.JakMod.LOG.warn("Airlock meshes missing",failure);}
            return airlock;
        }
        if(iris==null&&!irisMissing)try{
            // stand-in: tra-iris-door sets no open-sound or close-sound (only "blue-eco-on", played by the block);
            // sun-iris-door's "irisdoor2", Jak 1's own iris door, as it opens and as it closes.
            iris=leaf(RuntimeConfig.gameAssets("jak1").resolve("levels/training/jng-iris-door-lod0.glb"),"jng-iris-door","jng-iris-door-idle",3,
                new float[][]{{-1,1}},new float[][]{{0,1}},new String[]{"jak1/irisdoor2",null},new String[]{"jak1/irisdoor2",null},1);
            var frame=iris.model.poseAtTime(iris.clip,iris.model.duration(iris.clip));
            for(var blade:BLADES)frame=iris.model.withLocal(frame,blade,null,new Vector3f(1e-4f));
            irisFrame=frame;
        }catch(IOException failure){irisMissing=true;dev.jakmc.JakMod.LOG.warn("Iris door mesh missing",failure);}
        return iris==null?null:new Leaf[]{iris};
    }

    /** Where `ja :num! (seek! frame speed)` stretch after stretch has the clip `elapsed` seconds on from
     * `start`: the frame, and the stretch it is in (past the last: their count). */
    private static float[] run(float start,float[][] stretches,float frames,float elapsed){
        // The PC port's airlock speed (get-airlock-speed): Adrii 06/11 wants 2-3 s each way at most.
        float total=0,from=start;
        for(var s:stretches){float target=s[0]<0?frames:s[0];total+=Math.abs(target-from)/(s[1]*60);from=target;}
        float faster=Math.max(1,total/MOST_SECONDS);
        float frame=start;
        for(int i=0;i<stretches.length;i++){
            float target=stretches[i][0]<0?frames:stretches[i][0],speed=stretches[i][1]*60*faster;
            float need=Math.abs(target-frame)/speed;
            if(elapsed<need)return new float[]{frame+Math.signum(target-frame)*elapsed*speed,i};
            elapsed-=need;frame=target;
        }
        return new float[]{frame,stretches.length};
    }

    @Override public void extractRenderState(JakIrisDoor.Door door,State state,float partial,Vec3 camera,ModelFeatureRenderer.CrumblingOverlay breaking){
        BlockEntityRenderer.super.extractRenderState(door,state,partial,camera,breaking);
        state.poses=null;
        var block=door.getLevel()!=null?door.getLevel().getBlockState(door.getBlockPos()):door.getBlockState();
        if(!(block.getBlock() instanceof JakIrisDoor kind))return;
        door.seen(block.getValue(JakIrisDoor.OPEN));
        var leaves=leaves(kind.airlock);
        if(leaves==null)return;
        state.facing=block.getValue(BlockStateProperties.HORIZONTAL_FACING);
        boolean open=block.getValue(JakIrisDoor.OPEN);
        float elapsed=(System.nanoTime()-door.changedAt)/1e9f;
        state.leaves=leaves;state.poses=new GltfModel.Posed[leaves.length];
        for(int i=0;i<leaves.length;i++){
            var leaf=leaves[i];float frames=leaf.frames();
            // Never changed since it loaded: where it rests.
            if(door.changedAt==Long.MIN_VALUE/2){door.shown[i]=open?frames:0;door.from[i]=door.shown[i];}
            var at=run(door.from[i],open?leaf.opening:leaf.closing,frames,door.changedAt==Long.MIN_VALUE/2?0:elapsed);
            door.shown[i]=at[0];
            int step=(int)at[1];
            if(step!=door.step[i]&&door.changedAt!=Long.MIN_VALUE/2){
                door.step[i]=step;
                var sounds=open?leaf.openSounds:leaf.closeSounds;
                if(step<sounds.length&&sounds[step]!=null)play(door,sounds[step],leaf.gain);
            }
            // door-open: the iris open, its frame alone.
            state.poses[i]=!kind.airlock&&open&&at[0]>=frames?irisFrame:leaf.model.poseAtTime(leaf.clip,at[0]/60);
        }
    }
    private static void play(JakIrisDoor.Door door,String name,float gain){
        var level=door.getLevel();if(level==null)return;
        var at=door.getBlockPos().above();
        level.playLocalSound(at.getX()+.5,at.getY()+.5,at.getZ()+.5,SoundEvent.createVariableRangeEvent(JakData.id("sfx/"+name)),SoundSource.BLOCKS,gain,1,false);
    }

    @Override public void submit(State state,PoseStack stack,SubmitNodeCollector collector,CameraRenderState camera){
        if(state.poses==null)return;
        for(int i=0;i<state.leaves.length;i++)draw(state.leaves[i],state.poses[i],i,state.facing,stack,collector,state.lightCoords,1);
    }

    /** Leaf `index` (the airlock's second the inner, turned to face the other way) on the door's anchor. */
    private static void draw(Leaf leaf,GltfModel.Posed pose,int index,Direction facing,PoseStack stack,SubmitNodeCollector collector,int light,float alpha){
        stack.pushPose();
        stack.translate(.5,leaf.cells/2f,.5);
        stack.mulPose(new Matrix4f().rotationY((float)Math.toRadians(-facing.toYRot())));
        // Two cells across: the door's middle is half a block from its anchor, on the side of its other column.
        if(leaf.cells%2==0)stack.translate(.5,0,0);
        // Back to back: each door's own depth to its side of the middle.
        if(index==1)stack.mulPose(new Matrix4f().rotationY((float)Math.PI));
        if(leaf.clip.startsWith("com-airlock"))stack.translate(0,0,leaf.depth*leaf.thick*.5f);
        if(leaf.flat)stack.mulPose(new Matrix4f().rotationX((float)Math.PI/2));
        if(leaf.flat)stack.scale(leaf.fit,leaf.thick,leaf.fit);else stack.scale(leaf.fit,leaf.fit,leaf.thick);
        stack.translate(-leaf.middle.x,-leaf.middle.y,-leaf.middle.z);
        leaf.model.submit(pose,stack,collector,light,alpha);
        stack.popPose();
    }

    /** Placing one: the closed door, faint, where it would go (Adrii 06/11). */
    public static boolean ghost(JakIrisDoor.DoorItem item,net.minecraft.world.item.context.BlockPlaceContext context,Vec3 camera,PoseStack stack,SubmitNodeCollector collector,float alpha){
        if(!(item.getBlock() instanceof JakIrisDoor kind))return false;
        var state=kind.getStateForPlacement(context);
        if(state==null||item.plan(context,state)==null)return false;
        var leaves=leaves(kind.airlock);
        if(leaves==null)return false;
        BlockPos anchor=context.getClickedPos();
        stack.pushPose();
        stack.translate(anchor.getX()-camera.x,anchor.getY()-camera.y,anchor.getZ()-camera.z);
        for(int i=0;i<leaves.length;i++)draw(leaves[i],leaves[i].model.poseAtTime(leaves[i].clip,0),i,state.getValue(BlockStateProperties.HORIZONTAL_FACING),stack,collector,0xF000F0,alpha);
        stack.popPose();
        return true;
    }
}
