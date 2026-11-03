# Jak Worlds for Minecraft Java

Development port of the Jak systems from the Rust mashup into actual Minecraft Java. Targets Minecraft 26.3, Fabric Loader 0.20.1, Fabric API 0.165.0+26.3 and Java 25. MW2 and Skate 3 content are excluded; the original Jak JET-Board is included.

Minecraft owns registries, inventories, screens, entities, saves and collision shapes. A small versioned JNI library runs the existing pure Rust Jak simulation. Owned OpenGOAL exports provide the original models, animations, particle definitions and SFX; generated assets and all development saves are ignored by Git.

Current integration includes player rendering/movement, Morph Guns and partial powers, original sparticles/audio, persistent ammo/eco/board equipment, pickups, and 17 enemy types with original animation events. The full feature ledger is [docs/PORTING-SCOPE.md](docs/PORTING-SCOPE.md); the port is still in development. Exported city blueprints are not installed world generation yet. Vehicle/citizen/mount integration and remaining original presentation are ongoing.

Build with Java 25, Cargo and Gradle 9.8 (`gradlew build`). `buildNative` compiles the JNI library into the mod. The current native package is Windows x86-64. `RuntimeConfig` reads `config/jakmc.properties` (`assets=` points to the owned Jak 3 decompiler export; sibling Jak 1 exports are used for its creatures). Optional development overrides are `-Djakmc.assets` and `-Djakmc.native`.

`tools/import_audio.py` converts the owned OpenGOAL SFX banks with ffmpeg. The content exporter and `tools/import_content.py` prepare private block/object/blueprint resources. Source provenance and exact validation failures are recorded in [MODLOG.md](MODLOG.md). Retail game assets are not part of the source repository or a public release.

The Gradle development client uses `run/`, separate from the personal Minecraft installation. In that client, `/jak on` enables Jak and `/jak equipment` supplies equipment in creative mode. Original enemy spawn eggs are in the Jak creatures creative tab; vanilla `/summon jakmc:<type>` also works. Keyboard actions are under the Jak controls category. Controller and full vehicle controls are not integrated yet.

`tools/run_scene.py` drives bounded development scenes through an in-game file bridge, without desktop input. It records screenshots/results and closes its own client. Screenshots require visual review; native boundary checks and successful compilation do not certify untested gameplay. See `.local/evidence/` for private receipts.

Project code follows the source repository's Apache-2.0 license; the initial Fabric template's CC0 notice is retained in `LICENSE-FABRIC-TEMPLATE`. Extracted game content retains its original ownership.
