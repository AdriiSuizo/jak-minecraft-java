package dev.jakmc.client.model;

import org.joml.Matrix4f;
import org.joml.Quaternionf;
import org.joml.Vector3f;

/**
 * joint-mod-ik-callback (engine/anim/joint-mod.gc), the two-bone inverse kinematics of the pilot's arms (target-pilot-post sets
 * {@code arm-ik 0 and 1} to the vehicle's hand points). The callback sits on the elbow joint: its parent is the shoulder, and the hand
 * is {@code hand-dist} along the elbow's own Y axis. The elbow is bent by the law of cosines until the hand is as far from the
 * shoulder as the target is, then the shoulder swings the arm to point at it, then twists it about that line to keep the elbow on the
 * side it was on. {@code blend} (joint-mod-ik's own, 4 a second) scales both turns. No MC types: the math is tested on its own.
 */
final class ArmIk {
    private ArmIk(){}

    /** quaternion-from-two-vectors!: the shortest turn taking unit vector {@code a} to unit vector {@code b} (a half turn across them when opposite, where the original divides by zero). */
    static Quaternionf turnBetween(Vector3f a,Vector3f b){
        var cross=new Vector3f(a).cross(b);float length=cross.length(),dot=Math.clamp(a.dot(b),-1f,1f);
        if(length<1e-9f){
            if(dot>0)return new Quaternionf();
            var axis=(Math.abs(a.x)<.9f?new Vector3f(1,0,0):new Vector3f(0,1,0)).cross(a);
            return new Quaternionf().fromAxisAngleRad(axis.normalize(),(float)Math.PI);
        }
        float s=(float)Math.sqrt(.5*(1-dot))/length;
        return new Quaternionf(cross.x*s,cross.y*s,cross.z*s,(float)Math.sqrt(.5*(1+dot)));
    }
    /** quaternion-pseudo-slerp! from the identity: lerped (the near side) and normalised. */
    static Quaternionf share(Quaternionf q,float t){
        float sign=q.w<0?-1:1;
        return new Quaternionf(q.x*t*sign,q.y*t*sign,q.z*t*sign,(1-t)+q.w*t*sign).normalize();
    }
    /** vector-vector-angle-safe (vector.gc), its odd value for opposite vectors and all. */
    static float angleSafe(Vector3f a,Vector3f b){
        float dot=new Vector3f(a).normalize().dot(new Vector3f(b).normalize());
        if(dot>=1)return 0;
        if(dot<=-1)return .017257283f;
        return (float)Math.acos(dot);
    }
    /** vector-flatten! then vector-normalize!: {@code v} in the plane across {@code n}, as a unit vector. */
    static Vector3f flatten(Vector3f v,Vector3f n){
        var unit=new Vector3f(n).normalize();
        var out=new Vector3f(v).fma(-v.dot(unit),unit);
        return out.lengthSquared()>1e-18f?out.normalize():out;
    }

    /**
     * The shoulder's and the elbow's new world matrices, for the joints as {@code shoulder} and {@code elbow} are posed (model space),
     * a hand {@code handDist} along the elbow's Y and a {@code target} (model space); null when the arm has no length to solve with.
     * Everything under the elbow moves with it by {@code newElbow * elbow^-1}.
     */
    static Matrix4f[] solve(Matrix4f shoulder,Matrix4f elbow,float handDist,Vector3f target,float blend){
        var shoulderAt=shoulder.getTranslation(new Vector3f());var elbowAt=elbow.getTranslation(new Vector3f());
        var hand=new Vector3f(elbowAt).fma(handDist,new Vector3f(elbow.m10(),elbow.m11(),elbow.m12()));
        var upper=new Vector3f(elbowAt).sub(shoulderAt);
        var fore=new Vector3f(hand).sub(elbowAt);
        var whole=new Vector3f(hand).sub(shoulderAt);
        var toTarget=new Vector3f(target).sub(shoulderAt);
        float upperLength=upper.length(),foreLength=fore.length(),reach=toTarget.length();
        if(!(upperLength>1e-6f&&foreLength>1e-6f&&reach>1e-6f&&whole.lengthSquared()>1e-12f))return null;
        float bent=angleSafe(upper,fore);
        float wanted=(float)Math.acos(Math.clamp((reach*reach-upperLength*upperLength-foreLength*foreLength)/(2*foreLength*upperLength),-1f,1f));
        Vector3f axis,pole;
        if(bent==0){
            // The arm straight: the elbow's Z axis to bend about and its X axis for the side (elbow-rotation-axis 2, elbow-pole-vector-axis 0).
            axis=new Vector3f(elbow.m20(),elbow.m21(),elbow.m22());pole=new Vector3f(elbow.m00(),elbow.m01(),elbow.m02());
        }else{
            axis=new Vector3f(upper).cross(fore);
            pole=new Vector3f(upper).sub(new Vector3f(whole).mul(upper.dot(whole)/whole.dot(whole)));
        }
        pole.normalize();axis.normalize();
        var bend=new Quaternionf().fromAxisAngleRad(axis,wanted-bent);
        bend.transform(fore);
        whole.set(upper).add(fore);
        var swing=turnBetween(new Vector3f(whole).normalize(),new Vector3f(toTarget).normalize());
        swing.transform(upper);
        // Nearly straight at it: the side it was on stands in for the upper arm's own, for the twist below.
        if(Math.abs(new Vector3f(upper).normalize().dot(new Vector3f(toTarget).normalize()))>.98f)swing.transform(pole,upper);
        var twist=turnBetween(flatten(upper,toTarget),flatten(pole,toTarget));
        swing=new Quaternionf(twist).mul(swing);
        bend=share(bend,blend);swing=share(swing,blend);
        // The bend about the elbow where it is, then everything swung about the shoulder.
        var bentElbow=new Matrix4f().rotation(bend).mul(elbow).setTranslation(elbowAt);
        var pivot=new Matrix4f().translate(shoulderAt).rotate(swing).translate(-shoulderAt.x,-shoulderAt.y,-shoulderAt.z);
        return new Matrix4f[]{new Matrix4f(pivot).mul(shoulder),new Matrix4f(pivot).mul(bentElbow)};
    }
}
