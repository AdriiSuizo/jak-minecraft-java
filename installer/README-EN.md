# JakMod for Minecraft Java — installation

Development build for Windows 10/11 x86-64. Minecraft Java 26.3, Fabric Loader
0.20.1 and Fabric API 0.165.0+26.3. Requires all three Jak games installed in
OpenGOAL from your own copies. Not an official Mojang, Microsoft, Sony,
Naughty Dog or OpenGOAL product.

1. Launch Minecraft Java 26.3 once with the official launcher, then close it.
2. Install Jak and Daxter, Jak II and Jak 3 in OpenGOAL using ISOs from your own
   copies. Keep the installation and its extracted data. Once all three games
   are installed, you can store the original ISOs elsewhere and install JakMod.
   [OpenGOAL tutorial](https://www.youtube.com/watch?v=wYFE0wLbwic&t=2s)
3. Extract the **entire** RAR into a folder. Do not run the EXE inside the archive.
4. Close Minecraft and its launcher. Run `JakMod-Installer.exe`.
5. Choose **English**, check the OpenGOAL and Minecraft folders, and click
   **Install or update JakMod**. Use **Browse…** to change either path.
6. Wait for completion. Initial OpenGOAL extraction and conversion of cities,
   models and sounds may take several minutes. No Python, Rust, Gradle or
   administrator access is needed.
7. Reopen the launcher and select **26.3.jakmod**. If missing, open
   **Installations** and enable **Modded**.
8. Open **Jak Showcase (JakMod)** under **Singleplayer** to try the mechanics.
   The installer adds a copy of the test world and preserves your progress on updates.

Internet is used to download Fabric, Fabric API and the performance mods
included in the profile (Sodium, Sodium Extra, Lithium, FerriteCore and
EntityCulling). The installer converts your Jak resources locally.

The default OpenGOAL path is your detected installation, falling back to
`Documents\Jak OpenGOAL`. Select that folder or its `active` subfolder containing
`jak1`, `jak2` and `jak3`. Missing exports are generated using the installation's
extractor and the data already extracted to `data\iso_data`, inside the private Minecraft instance. The
installer does not download ISOs, modify OpenGOAL or use the MW2 repository.

JakMod worlds and settings live in `%APPDATA%\.minecraft\instances\jakmod`.
Updates preserve your worlds and your game and profile settings. Replaced files
are backed up under `instances\jakmod\jakmc\backups`. For a custom Minecraft folder, substitute its path for
`%APPDATA%\.minecraft`. Do not move or delete OpenGOAL after installing: the mod
continues reading its models and animations. Reinstall with the new path if
you move it. Extraction needs about 35 GB free on the Minecraft drive while installing if OpenGOAL does not already have the exports; it then removes what the mod does not use, leaving about 11 GB.

Jak and his original movement activate automatically when joining, including
worlds created with the earlier installer. The initial view is third person;
F5 and controller zoom still work. `/jak off` disables Jak and remembers your
choice. `/jak equipment` provides equipment in creative
mode. F5 changes perspective. Full controls are in `CONTROLES.md`; controller
bindings are available in game. This port is still in development; installation
does not certify complete parity or a particular frame rate.

On failure, click **Copy for a comment** in the error window. Paste the report
into the comments on the installation video on [AdriiSuizo's channel](https://www.youtube.com/@AdriiSuizo)
and describe what you were doing. **Open local log** shows the technical details
on your PC. Share the brief report to avoid exposing local paths. Share the
installer to distribute the mod. The generated JAR and extracted resources
include data from your games.

To uninstall, remove only the `26.3.jakmod` installation in the launcher and
its `versions\26.3.jakmod` folder. Keep `instances\jakmod` to preserve worlds.
Do not delete shared Fabric libraries.
