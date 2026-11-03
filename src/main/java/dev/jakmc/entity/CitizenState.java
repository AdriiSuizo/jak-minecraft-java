package dev.jakmc.entity;

import net.minecraft.world.entity.LivingEntity;

public interface CitizenState {
    int citizenVariant();
    void citizenVariant(int variant);
    int citizenPhase();
    long citizenPhaseStart();
    void citizenHit();
    void citizenAttacker(LivingEntity attacker);
    String citizenDefenseClip();
    long citizenDefenseStart();
    void citizenDefense(String clip,long start,boolean weaponOut);
    boolean citizenWeaponOut();
    void citizenWeaponOut(boolean value);
    String citizenDefenseShot();
    void citizenDefenseShot(String frame);
}
