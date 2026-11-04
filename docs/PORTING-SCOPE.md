# Porting scope

All Jak work is in scope. The order below is an implementation dependency order, not a reduction of the requested delivery.

| Group | Source oracle | Java integration |
|---|---|---|
| Jak/Daxter model, animations, eyes, cloth, hand attachments | assets/jak_model.rs, render_anim/jak_pose.rs | Minecraft render pipeline, interpolated skeleton |
| Movement, jumps, roll, ledge, ladder, swim, board/grind | jak_mode | batched native collision against Minecraft VoxelShapes |
| Melee, guns, dark/light powers, eco | jak_mode + render_anim/jak_eco.rs | server damage/projectiles, equipment and saved powers |
| Pickups, skull gems, sound and particles | jak_mode/particles, jak_skull_gems.rs | native entities, coherent model/emitter transforms |
| Citizens, guards, enemies, fauna and mounts | render_anim Jak systems | Minecraft entity lifecycle, navigation, water, beds, trades |
| Zoomers and HellCat | jak_mode/zoom, hellcat + render_anim | analog controls, seats, lights, effects and save |
| Dune Hopper and Sand Shark | wheeled_cars.rs | suspension, boost incl. air, 1-block steps, no decorative extra wings |
| Haven and Spargus | minecraft_terrain/jak_* | normal-world generation; desert-only Spargus; town surrounds tower |
| Tower, skate circuit, altar and palace | jak_palace_grid.rs, jak_city_arena.rs | modular Minecraft shapes, doors, lights, solid ramp placement |
| Props and bar | jak_objects*, jak_facades.rs | native blocks/recipes, selection matches shape, no invented emblems |
| Equipment and UI | actual Minecraft | board own saved slot, working creative search and normal vanilla screens |
| Visuals and compatibility | Java rendering plus OpenGOAL | stable occlusion, no clone fog/LOD overrides; native save/dimensions |

The user subsequently authorized the private repository and bilingual Windows installer. The private development release registers 26.3.jakmod and generates assets locally from owned OpenGOAL installations; game assets are not committed or distributed. Public publication has not been requested. Build, unit checks and visible real-game behavior have separate acceptance entries; none substitutes for the other.

Current actionable backlog: [FIXES-PENDIENTES.md](FIXES-PENDIENTES.md), with separate confirmed omissions, user reports and verification cases. Current implementation and remaining integration: [CURRENT-PORT-HANDOFF.md](CURRENT-PORT-HANDOFF.md). The original source references above remain oracles, not acceptance of completed Java features.

The user withdrew all custom lowpoly/far-terrain/far-tower work on 2026-11-05; those original-host features are out of scope for this Java version. F34–F36 are retired, not completed.
