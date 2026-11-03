# Jak → real Minecraft Java: current implementation

Updated 2026-11-03. Continue the full Jak port from the read-only original Rust mashup. Do not repeat recon or the accepted enemy/weapon takes without a relevant change. No MW2 content or Skate 3; Jak's JET-Board remains in scope. The user wants complete mechanics, effects, sound and normal Minecraft interaction, not only models.

## Projects and ownership

- Write/build/run only `C:/Users/vieir/Desktop/Repo/jak-minecraft-java` (own Git). Original Rust reference: `C:/Users/vieir/Desktop/Repo/2010-rust-rewrite-mashup-trabajo`, read-only. LSC4 and GTA SA are read-only workflow references; never build or instrument there.
- Minecraft 26.3, Fabric Loader 0.20.1, API 0.165.0+26.3, Loom 1.19.3, JDK 25, Gradle 9.8. Exact MC source jars live under the project's `.gradle/loom-cache/minecraftMaven`. This MC renderer uses SDL/RenderPearl; avoid stale GLFW/Blaze3D API assumptions.
- All launches use `run/`, with the owner's OpenGOAL exports under Documents/Jak OpenGOAL. No personal `.minecraft` profile/saves touched. Generated resources, retail audio/GLBs/PNGs, DLLs, saves and evidence are ignored and must stay out of Git/public distribution.
- Installer/publishing deferred by user. Own adapters/native source Apache 2.0, retained Fabric template license in LICENSE-FABRIC-TEMPLATE. No push/publish performed.

## Working checkpoint

- Original Jak pure Rust core, JNI live Minecraft VoxelShapes and actor collision, server-owned health/ammo/powers/inventory, original animation timings, keyboard + real SDL controller controls, own saved board slot, 12 Morph Gun items, native eco pickups.
- Native ABI **3**, **input_v3** adds right-stick axes; pose_v3 now carries ordered WeaponVisuals plus stable native-session/shot identifiers. Both Java and DLL must agree. Source-only checkpoint before weapon extension: `7d5bf09`.
- Original particle replay at 60 Hz, 588 parts/133 groups in selected libraries, original axes/UVs/blends, catch-up after asset-load stalls. 2695 unique original SFX clips / 7166 aliases imported locally, action/material cues and attached loops. This does not mean all game particle libraries, dialogue or music are integrated.
- 17 registered native MC enemy types: babak, lurkerpuppy, lurkercrab, bonelurker, swamp-rat, baby-spider, snow-bunny, yeti, hopper, double-lurker, grunt, flitter, gekko, mantis, kg-grunt, spydroid, predator. Actual navigation, collision, water/air/drowning, original clip timing/cues/hit reactions, crab shell logic, leap windup/landing, Minecraft combat damage and death/loot lifecycle. Five Metal Head kinds drop skull gems, including gekko/predator without visible gem joints. Original merc segment masks switch empty/filled gems for grunt/flitter/mantis.
- Original splash/foam/bubble effects attached to real MC water state. Body-bounds attack reach fixes whiffs after body separation. One-block chase/attack damages a real survival player.
- Fixed board/gun ancestor scale inheritance and original prejoint overrides, rather than arbitrary resizing. Folded board ~0.623 blocks vs deployed 1.94.
- Arc Wielder: compact authoritative rope points replay original six lightning-new bolts locally; original terminal/shock effects and BLUE_GUN2_LOOP. Source oracle matches every strip vertex across 30 frames. Mass Inverter rings now visible; continuous nonoverlapping annuli fix the rejected source billboard approximation. Shot meshes interpolate positions/rotations/scales/opacity with stable session identities and clear stale history.
- MC26.3 shutdown fix: reset level render data before integrated-server halt to cancel terrain uploads blocked on a full GPU upload buffer. One failed shutdown was traced and its exact dev PID stopped; subsequent runs save and exit normally. Do not touch unrelated Java processes.

## Evidence and verification

Read MODLOG.md for failures and exact receipts, not only successes.

- `.local/weapon-native-tests2.log`: 86 core + 5 JNI tests passed.
- `.local/weapon-presentation-check-final.log`: real JNI Arc 5120 bolt vertices + BLUE_GUN2_LOOP; Mass 6284 dark annulus vertices; stable saucer/grenade/Nova IDs; sample packets under 5 KB. Recompile standalone boundary tools after ABI changes.
- `.local/evidence/enemies-20261103-065748`: all 17 body/hurt/death captures and contact sheets reviewed, 5/5 gems, original cues, water/air/drowning and water geometry. Shutdown hung on this run; not a clean-exit acceptance.
- `.local/evidence/combat-20261103-072425`: accepted step/chase/attack damage + board hierarchy; normal save/exit. Water false fields mean not run in combat-only scene.
- `.local/evidence/weapons-20261103-075632`: failed dev camera take, deadline/no Jak draw. MC explicitly hides LocalPlayer when camera entity differs; no gameplay feature acceptance.
- `.local/evidence/weapons-20261103-080109`: 12-mode bounded render pass, Arc image + whole contact sheet reviewed. Mass billboard wedges rejected and corrected. Peacemaker actor tendrils not covered; shots can have flown past a static capture, so counters alone do not certify appearance.
- `.local/evidence/rings-20261103-080639`: corrected soft continuous annuli visually reviewed, normal save/exit.
- `tools/run_scene.py --scene ...` is bounded, bridge driven, uses no OS input. Its dev-only camera override is active only during WeaponScene or InputScene eco closeups. Movement checks use the real gameplay camera. Avoid leaving idle windows. DevBridge archives stale requests on startup.

## Still substantial work — not delivered yet

1. Citizens, guards/factions/trades/beds, fauna and mounts as real MC entities. Enemy special projectile attacks, articulated ragdolls, all original variants and source-specific responses are not complete.
2. Zoomers/HellCat/Jak 3 cars: host entities, controls, seats/entry/exit, collision streaming, wheels/suspension, lights, unlimited/air boost and save lifecycle. HellCat/flut pure core and particle definitions exist; that is not an integrated vehicle port. Zoom.rs is the gun camera zoom helper, not a zoomer vehicle implementation.
3. World generation Haven/Spargus/tower/palace/bar/props and functional blocks. Catalog/JBP export exists, but JakBlocks is deliberately not initialized: shapes/doors/light semantics are incomplete, and layered stone currently derives SnowLayerBlock (would melt). Implement proper native MC mechanics before enabling. No normal-world city generation yet. Spargus desert-only and town surrounding the tower remain requirements.
4. Skull gem original animated material/halo/collect burst/spawn/bounce/suction cues; original pickup magnetics. Real MC item collision/save/pickup exists, but the static GLB is not the full gem effect. Eco groups render, light eco scale .25. Dropped eco billboards at four angles and both held variants reviewed after camera/item-transform fix.
5. Complete Wave mesh/scorch/warp, screen distortion/heat haze, terrain-projected gravity shadows, more source particle libraries. Current gravity rings are planar like the old adapter; original shadow projection is not implemented.
6. Daxter eyes/cloth and full source pose selection, generic held item material/attachment refinements, host survival/knockback/portal/pressure-plate coherence, ladder/rail metadata for native movement.
7. Controller GUI cursor/grid navigation and the user's vehicle R2 throttle/L2 brake-reverse/X height/HellCat action/Square handbrake layout remain. On-foot SDL control, both sticks, analog triggers, button edges, Triangle vanilla use, perspectives, inventory open/close and hotbar are integrated; see CONTROLES.md.
8. Camera occlusion refinement, dialogue/music, long-biome vehicle profiling and 150 FPS target. Do not claim those from startup FPS or simulation unit checks.

Continue with substantive integration, keeping source-oracle mechanics and actual Minecraft entity/block lifecycle together. Existing tests are evidence to reuse, not a reason to restart the audit.

## 2026-11-03 controller / eco checkpoint

- Minecraft 26.3 bundles LWJGL SDL 3.4.3. SdlGamepad uses SDL_GetGamepads/OpenGamepad/GetGamepadAxis, frees the native ID buffer, closes hot-unplugged handles and balances its subsystem reference. No driver, key emulation or GLFW dependency. Original Bevy/native pad rules are traced in controller-source-provenance.json.
- Camera polls before frame extraction and calls Entity.turn with its .15 factor accounted for; native server packets now receive both sticks and pedals. Server view yaw/pitch are separate from native body heading. Menus/focus suppress gameplay and do not synthesize attacks when reopened.
- Native pose adds state name for evidence. A double jump deliberately shares the source jump clip: assert `double-jump` state rather than inventing another clip. No core physics changes in this checkpoint.
- `.local/controller-boundary-states.log`: physical Xbox One Controller detected, actual SDL virtual input mapped through real JNI to walk/jump/double-jump/roll/attack/board; peak 1.97952 blocks. Source deadzones, analog conversion, release and hot-unplug passed.
- `.local/evidence/input-20261103-142053`: actual SDL → normal client input → input_v3 → server Jak, no probeInput. Walk 1.4033 blocks; jump peak 1.98592; yaw delta 67.8842 degrees; all four required action states. Original animation/FX screenshots and four dropped-eco angles, both hand items and vanilla creative GUI inspected. Normal save/exit.
- `.local/evidence/input_actions-20261103-144140`: real Triangle equips the saved board, all three perspectives, D-pad opens vanilla creative and Start closes it; all four assertions pass, normal exit. Earlier interaction takes failed because replay began during LevelLoadingScreen / before SDL hot-plug enumeration and selected-slot sync. The runner now waits for loaded world; replay waits until the actual backend opens the device at rest. Failed takes retained, no gameplay bypass.
- EcoItemRenderer used a fixed local camera despite item spin/bob/hand transforms. It now transforms the true camera through the inverse complete item pose. GUI uses an explicit icon context. This fixes the edge-on needle/plane in the user's two screenshots; selected original group sizes stay light .25 / dark .5. It does not close all particle-library/rendering gaps.
- Before test-world changes: `.local/backups/pre-controller-20261103-141935.zip`. Personal Minecraft, old source, LSC4 and GTA SA remain untouched. No automatic test runs during PROBAR-JAK manual play.

Final controller/eco build: .local/controller-final-build.log (2m17), 86+5 native tests pass, packaged DLL/source-archive receipt .local/controller-build-receipt.json; final DLL SDL→JNI replay .local/controller-packaged-jni-check.log passes. Source checkpoints cf8ce78 (controller) and 6826c91 (eco view). No dev client left running.
