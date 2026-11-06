package dev.jakmc.citizens;
/** The town life of a citizen (CitizenLifeMixin on the native Villager); its state and start are synced for the renderer. */
public interface CitizenLifeState {
 CitizenLife citizenLife();
 int citizenLifeState();
 long citizenLifeStart();
 void citizenLifeState(int state,long start);
}
