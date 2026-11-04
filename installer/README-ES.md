# JakMod para Minecraft Java — instalación

Versión de desarrollo para Windows 10/11 de 64 bits. Minecraft Java 26.3,
Fabric Loader 0.20.1 y Fabric API 0.165.0+26.3. Requiere los tres juegos Jak
instalados con OpenGOAL desde tus propias copias. No es un producto oficial
de Mojang, Microsoft, Sony, Naughty Dog ni OpenGOAL.

1. Abre Minecraft Java 26.3 una vez desde el launcher oficial y ciérralo.
2. Instala Jak and Daxter, Jak II y Jak 3 en OpenGOAL. Conserva la instalación.
3. Extrae **todo** el RAR en una carpeta. No ejecutes el EXE desde dentro del RAR.
4. Cierra Minecraft y su launcher. Ejecuta `JakMod-Installer.exe`.
5. Elige `es`, confirma la carpeta de OpenGOAL detectada y la de Minecraft,
   o pulsa **Examinar…** para cambiarlas. Pulsa **Instalar JakMod**.
6. Espera a que termine: la primera extracción de OpenGOAL y conversión de
   ciudades, modelos y sonidos puede tardar varios minutos. No necesita Python,
   Rust, Gradle ni permisos de administrador.
7. Reabre el launcher y selecciona **26.3.jakmod**. Si no aparece, entra en
   **Instalaciones** y activa **Modificadas**.

Internet se utiliza para descargar Fabric y Fabric API desde sus servidores.
No se suben tus archivos de juego ni datos de tu cuenta. Los recursos de Jak
se convierten en tu ordenador; la descarga no incluye los juegos.

La ruta propuesta para OpenGOAL es tu instalación detectada o, si no se detecta,
`Documentos\Jak OpenGOAL`. Se acepta esa carpeta o su subcarpeta `active`, con
`jak1`, `jak2` y `jak3` dentro. Si faltan exportaciones, se usa el extractor de
esa instalación y sus datos ISO locales para generarlas en la instancia privada.
El instalador no descarga ISOs, no modifica OpenGOAL y no usa el repositorio MW2.

Las partidas y configuración de JakMod están en
`%APPDATA%\.minecraft\instances\jakmod`. Los mundos normales no se borran.
Se guardan copias locales de los perfiles del launcher antes de modificarlos.
Para otra carpeta de Minecraft, sustituye la parte `%APPDATA%\.minecraft`.
No muevas ni borres tu instalación de OpenGOAL después de instalar: el mod
sigue leyendo sus modelos y animaciones. Reinstala indicando la nueva ruta
si la cambias. La extracción necesita espacio libre adicional.

Jak y su movimiento se activan automáticamente al entrar, también al actualizar
partidas creadas con el instalador anterior. Empiezas en tercera persona; F5 y
el zoom del mando siguen funcionando. `/jak off` permite desactivarlo y se
recuerda tu elección. En creativo, `/jak equipment`
proporciona equipo. F5 cambia la cámara. Los controles completos están en
`CONTROLES.md`; el mando se configura en el juego. El port sigue en desarrollo;
el instalador no certifica paridad completa ni un rendimiento concreto.

Si falla, conserva el mensaje y los registros de
`instances\jakmod\jakmc\setup`. No los publiques sin revisarlos: contienen rutas
locales. No compartas el JAR final generado, los recursos extraídos ni las copias
de perfiles del launcher: comparte este instalador sin archivos de juego.

Para desinstalar, elimina únicamente la instalación `26.3.jakmod` desde el
launcher y su carpeta `versions\26.3.jakmod`. Conserva `instances\jakmod` si
quieres guardar tus mundos. No borres bibliotecas compartidas de Fabric.
