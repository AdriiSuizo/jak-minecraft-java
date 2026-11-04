# JakMod for Minecraft Java — installation

Development build for Windows 10/11 x86-64. Minecraft Java 26.3, Fabric Loader
0.20.1 and Fabric API 0.165.0+26.3. Requires all three Jak games installed in
OpenGOAL from your own copies. Not an official Mojang, Microsoft, Sony,
Naughty Dog or OpenGOAL product.

1. Launch Minecraft Java 26.3 once with the official launcher, then close it.
2. Install Jak and Daxter, Jak II and Jak 3 in OpenGOAL. Keep that installation.
3. Extract the **entire** RAR into a folder. Do not run the EXE inside the archive.
4. Close Minecraft and its launcher. Run `JakMod-Installer.exe`.
5. Choose `en`, confirm the detected OpenGOAL and Minecraft folders, or use
   **Browse…** to change them. Click **Install JakMod**.
6. Wait for completion. Initial OpenGOAL extraction and conversion of cities,
   models and sounds may take several minutes. No Python, Rust, Gradle or
   administrator access is needed.
7. Reopen the launcher and select **26.3.jakmod**. If missing, open
   **Installations** and enable **Modded**.

Internet is used to download Fabric and Fabric API from their servers. Your
game files and account information are not uploaded. Jak resources are
converted on your PC; this download does not include the games.

The default OpenGOAL path is your detected installation, falling back to
`Documents\Jak OpenGOAL`. Select that folder or its `active` subfolder containing
`jak1`, `jak2` and `jak3`. Missing exports are generated using the installation's
extractor and local ISO data, inside the private Minecraft instance. The
installer does not download ISOs, modify OpenGOAL or use the MW2 repository.

JakMod worlds and settings live in `%APPDATA%\.minecraft\instances\jakmod`.
Your normal worlds are kept. Launcher profiles are backed up locally before
changes. For a custom Minecraft folder, substitute its path for
`%APPDATA%\.minecraft`. Do not move or delete OpenGOAL after installing: the mod
continues reading its models and animations. Reinstall with the new path if
you move it. Extraction requires additional free disk space.

Jak and his original movement activate automatically when joining, including
worlds created with the earlier installer. The initial view is third person;
F5 and controller zoom still work. `/jak off` disables Jak and remembers your
choice. `/jak equipment` provides equipment in creative
mode. F5 changes perspective. Full controls are in `CONTROLES.md`; controller
bindings are available in game. This port is still in development; installation
does not certify complete parity or a particular frame rate.

On failure, keep the message and logs under `instances\jakmod\jakmc\setup`.
Review logs before sharing: they contain local paths. Do not share the generated
JAR, extracted resources or launcher-profile backups. Share this asset-free
installer instead.

To uninstall, remove only the `26.3.jakmod` installation in the launcher and
its `versions\26.3.jakmod` folder. Keep `instances\jakmod` to preserve worlds.
Do not delete shared Fabric libraries.
