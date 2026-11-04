# Jak Worlds for Minecraft Java

Development port of the Jak systems from the Rust mashup into actual Minecraft Java. Targets Minecraft 26.3, Fabric Loader 0.20.1, Fabric API 0.165.0+26.3 and Java 25. MW2 and Skate 3 content are excluded; the original Jak JET-Board is included.

Minecraft owns registries, inventories, screens, entities, saves and collision shapes. A small versioned JNI library runs the existing pure Rust Jak simulation. Owned OpenGOAL exports provide the original models, animations, particle definitions and SFX; generated assets and all development saves are ignored by Git.

Current integration includes the cinematic Jak/Daxter models, cloth, original movement and combat, Morph Guns, Dark/Light powers, original particles/audio, inventory-owned eco/ammunition, durable JET-Board equipment, native citizens/trades and 17 enemy types. Haven, desert-only Spargus, Sandover and the crater cave generate in normal worlds. Rideable Leapers/Flut Fluts, HellCat/Helldog, zoomers and two Jak 3 wheeled cars use the source simulation. The port is still in development; [docs/CURRENT-PORT-HANDOFF.md](docs/CURRENT-PORT-HANDOFF.md) records current evidence and remaining limits, and [docs/PORTING-SCOPE.md](docs/PORTING-SCOPE.md) retains the feature inventory.

Build with Java 25, Cargo and Gradle 9.8 (`gradlew build`). `buildNative` compiles the JNI library into the mod. The current native package is Windows x86-64. `RuntimeConfig` reads `config/jakmc.properties` (`assets=` points to the owned Jak 3 decompiler export; sibling Jak 1 exports are used for its creatures). Optional development overrides are `-Djakmc.assets` and `-Djakmc.native`.

`tools/import_audio.py` converts the owned OpenGOAL SFX banks with ffmpeg. Run `python tools/import_flut_eyes.py <owned-jak1-eye-layers>` with Pillow to bake the three source eye layers from the Rust project's `crates/assets/data/jak1-eyes` folder. Run `python tools/import_hellcat_deck.py <owned-jak3-export>` to import the HellCat collision relief and authored damage partitions. Pass `--helldog` with the owned Jak II export to import the black Hell Dog hull. The content exporter and `tools/import_content.py` prepare private block/object/blueprint resources. Source provenance and exact validation failures are recorded in [MODLOG.md](MODLOG.md). Retail game assets are not part of the source repository or a public release.

The Gradle development client uses `run/`, separate from the personal Minecraft installation. In that client, `/jak on` enables Jak and `/jak equipment` supplies equipment in creative mode. Original enemy spawn eggs are in the Jak creatures creative tab; vanilla `/summon jakmc:<type>` also works. Keyboard actions are under the Jak controls category. SDL controller input now drives the same original movement core in first and third person. Native inventory screens retain Minecraft behavior, with controller input and cursor navigation; see [CONTROLES.md](CONTROLES.md).

`tools/run_scene.py` drives bounded development scenes through an in-game file bridge, without desktop input. It records screenshots/results and closes its own client. Screenshots require visual review; native boundary checks and successful compilation do not certify untested gameplay. See `.local/evidence/` for private receipts.

Project code follows the source repository's Apache-2.0 license; the initial Fabric template's CC0 notice is retained in `LICENSE-FABRIC-TEMPLATE`. Extracted game content retains its original ownership.

## Local manual play

On this machine, double-click `PROBAR-JAK.bat`. It sets Java 25 only for its own process and runs the isolated Fabric development client; it does not install into the personal Minecraft profile. The configured dev world opens automatically. No automatic test scene starts.

Use `/gamemode creative`, `/jak on`, `/jak equipment`, then F5 for third person. Right-click the JET-Board once to equip it into its saved slot, then B to mount/stow. Hold a Morph Gun and press R to fire; hold/release R to charge/fire the Wave Concussor. G punches, V spins, Space jumps. Creature eggs are in the Jak creatures creative tab; `/summon jakmc:grunt ~ ~ ~4` is a direct example. Switch to survival to test enemy aggression. Z/X use dark/light powers when sufficient eco is available. Controls can be rebound in Minecraft options.

Controller. Left stick moves and right stick looks. PlayStation X / Xbox A jumps and double-jumps, Square / X punches and Circle / B spins. L1 rolls with movement, L2 aims and R2 fires or operates the equipped Minecraft item or JET-Board. Triangle / Y uses Minecraft items, doors and entities. Select / Back opens inventory and Start pauses. Hold R3 with D-pad Up or Down to zoom into or out of first person. Tap D-pad Down to drop one item; hold it to drop the whole stack. L3 controls transformations and Light Jak chords. See [CONTROLES.md](CONTROLES.md) for the complete mapping.


Eco y munición se consumen del inventario real, incluidas shulkers y bolsas. Las recetas, la durabilidad y los comercios se explican en [Inventario y economía](docs/INVENTORY-ECONOMY.md). Para localizar las ciudades de generación normal: `/jak locate haven` y `/jak locate spargus`.
