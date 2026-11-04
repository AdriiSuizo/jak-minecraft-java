# Build-only construction converter

This snapshot makes `tools/content-exporter` independent of the original Rust
mashup checkout. It contains the project's Rust construction generators and
their MinecraftOSS data-type dependencies, not the old host or its game assets.
It is used only during content import; Minecraft Java remains the game host.

Source: the owner's `2010-rust-rewrite-mashup-trabajo` construction adapters,
isolated and made portable on 2026-11-04. The Java-port exporter already applied
the public-entry-point changes documented in `tools/prepare_source_export.py`.
Project Apache-2.0 terms and attribution are retained in LICENSE.

No original textures, audio, GLBs, raw objects or decompiled GOAL files are
included. Two former embedded retail-derived inputs are now required at runtime:
`JAKMC_SKULL_ICON` is a PNG from the owner's OpenGOAL texture export;
`JAKMC_SUPPORT_PROFILE` is regenerated from their Jak II palace GLB by
`tools/extract_skate_tower_support.py`. `IW4L_JAK1_ASSETS`,
`IW4L_JAK2_ASSETS` and `IW4L_JAK3_ASSETS` point at owned local exports.

The exporter writes only to its selected output folder. `MINECRAFTOSS_ROOT`
provides local Minecraft blockstate/model templates, extracted from the player's
installed Java client. They are not part of this source snapshot or installer.
