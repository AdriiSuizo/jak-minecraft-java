![Jak Worlds](docs/media/jak-worlds-header.gif)

[Inicio](README.md) · [Descargar](https://github.com/AdriiSuizo/jak-minecraft-java/releases/tag/v0.1.0-dev.19) · [Guías](docs/README.md)

---

# Handoff — Jak en Minecraft Java

Actualizado el **5 de noviembre de 2026, 02:42 CET (Europe/Zurich)**. Este es el punto de entrada para continuar. Lee este archivo completo y después los enlaces pertinentes; no reinicies toda la auditoría ni repitas pruebas aceptadas sin una modificación o un fallo concreto que lo justifique.

## 1. Petición actual y alcance

El trabajo es llevar a **Minecraft Java real** el contenido y las mecánicas de Jak que el usuario añadió al mashup Rust, con sus correcciones, modelos, animaciones, ropa física, efectos, sonidos, criaturas, monturas, vehículos, ciudades y lógica. Aprovechar OpenGOAL y la evidencia del origen; adaptar a las reglas e interfaces de Minecraft cuando corresponda. No basta con mostrar modelos.

- **Excluir MW2 y Skate 3.** La turbotabla/JET-Board de Jak sí está incluida.
- **Lowpoly propio retirado por el usuario el 05/11:** no implementar terreno lejano, torre lejana ni mejoras del lowpoly Rust en Java. F34–F36 están retirados, no terminados. La petición posterior de ver 32 chunks se refiere a la distancia normal de Minecraft; no revoca esta retirada.
- La tabla se saca/guarda con **Triángulo/Y**, únicamente si está equipada en su ranura. Llevarla en la mano o en otra casilla no basta. R2 no despliega la tabla.
- L2/LT usa/interactúa mediante Minecraft; con Morph Gun también apunta. R2/RT ejecuta ataque/minado/acción primaria o dispara el Morph Gun. Los gatillos de conducción conservan acelerador y freno.
- No convertir decisiones de agentes anteriores en restricciones arbitrarias. Mantener las correcciones útiles; distinguir un fallo heredado del origen de una omisión de este port.
- El usuario pidió continuidad pragmática: resolver una integración antes de saltar a otra, no afirmar paridad completa por contadores o capturas sin revisar, no dejar pruebas abiertas ni molestar los otros proyectos.
- **Último encargo: preparar este handoff.** No se ha cambiado código ni configuración de Minecraft durante su preparación.
- La petición de dar permisos de GitHub a Claude fue **retirada inmediatamente** con «ignora el último mensaje». No se han cambiado permisos, colaboradores ni credenciales. No es un pendiente autorizado.

## 2. Carpetas, límites y herramientas

| Uso | Ruta |
|---|---|
| Proyecto activo, escritura/build | `%USERPROFILE%/Desktop/Repo/jak-minecraft-java` |
| Origen Rust de referencia, solo lectura | `%USERPROFILE%/Desktop/Repo/2010-rust-rewrite-mashup-trabajo` |
| OpenGOAL del propietario, solo lectura | `%USERPROFILE%/Documents/Jak OpenGOAL/active` |
| Instancia real del launcher | `%USERPROFILE%/AppData/Roaming/.minecraft/instances/jakmod` |
| Espacio de trabajo de este chat | `%USERPROFILE%/Documents/Codex/2026-10-25/contin-a-el-trabajo-jak-minecraft` |

No escribir, compilar, instrumentar ni ejecutar pruebas en LSC4/GTA SA/GTA IV. Solo son referencias de metodología cuando sea pertinente. No modificar directamente partidas personales. El mundo normal de la instancia se llama `Mundo nuevo` y sigue presente.

Hay skills instaladas en `%USERPROFILE%/.codex/skills`: `mod-any-game`, `mashup-mods`, `game-automation`, `publish-mod`, etc. Leer las que se apliquen al siguiente trabajo; no repetir el reconocimiento del juego desde cero. Las normas de porting del escritorio son filosofía de trabajo, no una orden para trabajar en los repositorios de GTA.

Entorno: Minecraft **26.3**, Fabric Loader **0.20.1**, Fabric API **0.165.0+26.3**, Loom **1.19.3**, Gradle **9.8**, JDK **25**, Rust/Cargo. Esta versión usa SDL/RenderPearl: no aplicar sin comprobar APIs viejas de GLFW/Blaze3D. Fuentes Minecraft locales en `.local/mc-api`, `.local/sources` y caché Loom. Consultar primero esas fuentes para dudas de API.

## 3. Estado exacto de código y entrega

Repositorio **privado**: https://github.com/AdriiSuizo/jak-minecraft-java

- Código instalado: **`b8b39488452a5b888a35a4e31f6f0ed356bd87a7`**.
- Último commit antes de preparar este documento: **`602e9d2`**, solo recibos/documentación.
- Release privado instalado: https://github.com/AdriiSuizo/jak-minecraft-java/releases/tag/v0.1.0-dev.3
- Java/Rust **ABI 5**. No mezclar clases nuevas con DLL de ABI anterior. La caché de DLL está identificada por SHA-256.
- Perfil del launcher: **`26.3.jakmod`**. Versión interna del mod: `0.1.0-dev`.
- RAR local: `dist/JakMod-26.3-0.1.0-dev.rar`, **176.607.364 bytes**.
- SHA-256 del RAR: `7cac4d6c1d54a8a02ff58431b36e820ffaac417c88893f34f193019d890a9ab8`. El digest de GitHub coincide.
- Instalador congelado ejecutado realmente, proceso propio 8572 ya terminado. Recibo: **2026-11-05T00:54:43.954177Z**, equivalente a **01:54:43 CET**.
- Recursos locales generados: **13.120**. Audio: **2.815 clips únicos / 7.328 eventos**.
- SHA-256 del JAR instalado: `2899bd6cea16b77b1ee1d865131ec773ae15eadce266e307f07acb49c540ba0c`.
- Las **461 entradas de código** del payload exacto coinciden byte a byte con el JAR instalado. Evidencia: `.local/evidence/vehicle-hurt-installed-code-check.json`.
- Auditoría de distribución: **923 archivos de fuente**, JAR de código **1.996.975 bytes**, **cero** intersecciones con recursos privados, sin fallos. `.local/evidence/vehicle-hurt-distribution-audit.log`.
- GitHub CI del código **41944144808: success/completed**, vuelto a comprobar al preparar este handoff. El recibo antiguo de MODLOG todavía describía ese CI en curso; ese estado ya ha quedado superado.

Instalador/RAR ES/EN solicitan la ruta de OpenGOAL, proponen rutas detectadas y generan recursos localmente. Ni Git ni los paquetes incluyen recursos retail. El JAR fusionado dentro de la instancia sí contiene recursos extraídos del propietario y **no se distribuye**. Mantener privados GitHub y las releases; no se ha autorizado publicación pública. Guardar copias y preservar perfiles, configuración, mundos e inventarios al actualizar.

## 4. Cambios ya integrados en esta entrega

### Controles, agua y daño al mundo — base `55fe405`

- Eliminada la ruta que adivinaba acciones por categorías de objetos y desplegaba BOARD por defecto. Triángulo requiere la casilla de tabla tanto en cliente como en servidor. L2 sigue el uso nativo, incluyendo mano secundaria y mantener/soltar; R2 ataque/minado o Morph Gun. Se conserva R1 para trucos de tabla.
- Salto desde agua: la separación de suelo consumía casi toda la altura a la escala Java. Regresión antes: pies solo **0,001476 bloques** sobre agua. Salto simple aceptado después: **0,450154 bloques**, sin doble salto. No equivale a comprobar todas las orillas.
- Monturas: dimensiones conjuntas del animal/jinete pasan al núcleo; posición resuelta mediante `Entity.move`, no `setPos` atravesando bloques, y se devuelve la corrección al simulador. Flut Flut comprobado frontalmente sin penetración ni daño falso; faltan casos específicos del ledger.
- `JakWorldDamage` consume Strike/Impact/WorldHit: dureza, grietas acumuladas, caducidad, reglas del jugador y drops. Explosiones usan resistencia/agua/callbacks de Minecraft sin duplicar daño o empuje de actores, ni FX/audio nativos.
- Onda Hellcat/Helldog reducida de 60 a 12 metros fuente (**7,68 bloques**), con obstrucción por paredes y un impacto por actor. Conserva anillos originales. Dispatch de terreno para cuatro explosiones Morph Gun, naves y armamento de coches.

### Colisiones de vehículos y daño/presentación — `b8b3948`

- Zoomers/coches: barrido continuo de caja orientada contra formas reales de bloques. Cubre las esquinas diagonales que dejaban libres las tres esferas/cajas anteriores, sin bloquear el espacio vacío del rectángulo envolvente. Usa el rumbo recién integrado y rechaza giros de un casco libre hacia una pared.
- El casco de coches conserva el perfil vertical del simulador original; no declarar certificada cualquier inclinación posible por las pruebas de SAT horizontal.
- Hellcat/Helldog: esferas, cubierta, límites y punto de salida siguen la actitud completa. Barrido angular y validación de arco slerp para impedir girar parado dentro de una pared. Se conserva suspensión/paso de un bloque, turbo aéreo y controles existentes.
- Omisión corregida: el daño Minecraft no invocaba `Jak::hurt`. Ahora se encola fuera de callbacks JNI y traduce modos originales: golpe, explosión, fuego, electricidad, daño sin golpe y caída. Reacciones, Daxter, partículas, OOF/GET_BURNED y buzz originales; no vuelve a descontar salud. Se evita duplicar el sonido vanilla de daño para Jak activo.
- Temporizadores originales de 300 Hz alimentan parpadeo y tinte de quemadura/electricidad en el reloj de render compartido. Cuerpo, Daxter y ropa capturan tintes inmutables para la geometría diferida; retratos de inventario neutros. No confundir esto con resolver toda la presentación de muerte.
- SDL consume buzz en el mando controlador: motor fuerte 0,5 s para golpes, débil 0,1 s para caída. Se detiene al perder foco, entrar a menú, cambiar dispositivo o cerrar. Un mando sin rumble falla de forma tolerada. Validado el callback SDL con pad virtual propio; no se afirma prueba táctil de cada mando físico.

## 5. Evidencia aceptada y errores que no se deben repetir

Todo lo siguiente es relativo a `.local/evidence/`; son recibos locales ignorados por Git:

| Evidencia | Qué cubre |
|---|---|
| `integration-20261105-003848` | Salto simple de agua, Flut frontal, Blaster rompe muro, vegetación presente antes de explotar, drops, bedrock, sin daño/impulso duplicado |
| `integration-20261105-005746` | Lo anterior más 24 límites geométricos: morro, hueco exterior y barrido continuo en ocho vehículos |
| `cars-20261105-010230` | Cuatro ruedas en ambos coches, subida de un bloque, velocidades de paso 22,38/17,72 m/s fuente, turbo aéreo +6,955, armas y bajada |
| `zoomers-20261105-010611` | Recorridos, luces, alturas, marcha atrás y soltar stacks manteniendo abajo |
| `hellcat-20261105-011500` | Las 12 acciones de su fixture, subida, turbo en X, luces nocturnas, salida encendida y patada de apagado |
| `helldog-20261105-010822` | Las 12 acciones de su fixture para la otra nave |
| `input_actions-20261105-014423` | Las 22 aserciones: ranura/Triángulo, separación L2/R2, Select/R3/menús, daño, ambos motores, parpadeo y tintes de fuego/electricidad |
| `hurt-visuals-native.log` | **96 pruebas core + 7 JNI** aprobadas |
| `vehicle-hurt-package-build.log` | `build check distributionJar` correcto |

Cuatro pruebas de límites del instalador aprobadas. Las pruebas Java de Gradle son `NO-SOURCE`; no sumar las comprobaciones de escenas como si fueran pruebas unitarias Java. Se revisaron capturas diurnas de daño, suspensión y vuelo/salida con luces. Ninguna de estas escenas demuestra **150 FPS sostenidos** ni paridad total.

Errores/limitaciones conservados en los recibos:

- El primer intento de salto perdía una pulsación por planificación cliente/servidor; el siguiente incluía doble salto. El fixture aceptado emite un único borde en `START_SERVER_TICK`.
- Un fixture tenía hierba sobre hojas y no comprobaba que existiese antes de explotar; corregido a tierra + presencia previa.
- La colisión angular por cuerda no equivalía al arco de la rotación. La regresión detectó penetración; se añadió validación real del arco.
- Un pulso débil se medía después de acabar sus 0,1 s. Ahora se registra el callback no nulo. El shock se exige dentro de su ventana de render, no en una única observación anterior al snapshot.
- Una supuesta pérdida de selección con zoom no resultó ser un bug demostrado de R3: venía del estado de la partida copiada/selección inicial del fixture. No hubo cambio de runtime de zoom; se comprobó conservación de casilla. No volver a afirmar esa causa sin evidencia nueva.
- Las capturas con objetos heredados del hotbar no certifican el aspecto de todos los objetos en la mano.
- La primera llamada PowerShell al EXE GUI volvió antes de que terminase y mostró un recibo viejo. **No aceptar un log/recibo antiguo como prueba de instalación.** Esperar al proceso propio, comprobar timestamp nuevo y comparar el payload.
- El primer SAT agotó el plazo antes de lanzar el juego, por build/copia de recursos. El runner separa prebuild y arranque. No ampliar a ciegas el plazo de un juego colgado.
- Algunas escenas incluyen campos de otras modalidades sin ejecutar: consultar el fixture antes de interpretar `false` como fallo o éxito.

## 6. Pendientes reales: continuar aquí

Lista autoritativa con evidencia, código y criterios: [docs/parity/pendientes.json](docs/parity/pendientes.json) y [docs/FIXES-PENDIENTES.md](docs/FIXES-PENDIENTES.md). Hay **33 entradas**, de las que **F28 está resuelta**, y **32 abiertas/parciales**. Además: **11 contrastes V01–V11**, **3 incompletos heredados H01–H03** y **3 retirados F34–F36**. No son 32 ausencias completas: varias integraciones ya están implementadas y falta contrastar variantes específicas.

Orden útil desde la evidencia existente:

1. F01–F17: completar casos de agua/orillas, contactos de monturas/direcciones, ciclo de destrucción y alcance real del daño melee/armas/naves, interacción de objetos. No rehacer los puentes ya aceptados.
2. **F18**: presentación de uso de objetos Minecraft en la mano de Jak. **F19**: calibración de manos del agarre con cuerpo cinematográfico. **F20–F22**: ragdoll Grunt articulado real, reacciones por especie, volúmenes y drops durante caída.
3. F23–F24: multitudes/rutas, nidos/bandas/reposición. F25–F26: poderes/cuerpo en primera persona fuera del Morph Gun, distorsión/composición de efectos.
4. **F27**: sonidos originales de animación de habitantes. La etiqueta inicial «voces y diálogos» era una inferencia sin respaldo. `crates/render_anim/src/jak2_villagers.rs` del origen contiene seis cues de pasos, aterrizaje y bastón (`was-*`/`wasf-*`); defensa ya consume cues, caminar/recuperarse todavía requiere integración. **No importar voces MW2 ni inventar conversaciones para cerrar la lista.**
5. F29–F33: catálogo no activo, variedad procedural/ciudades, regiones/vegetación, iluminación/materiales y rendimiento sostenido. El catálogo tiene 83 objetos candidatos no activos; alias/materiales no deben contarse como cientos de funciones independientes.

Pistas ya investigadas, sin implementación nueva:

- Grunt original: `crates/render_anim/src/grunt_ragdoll.rs`, 47 nodos, restricciones y pasos de 1/120 s. Adaptar solver y pose reales; no sustituirlo por inclinar todo el cuerpo 90 grados ni por un cadáver solo visual. Mantener autoridad de daño/drop y colisión del servidor. `GltfModel.Posed` expone matrices/joints útiles.
- Ciudadanos: `CitizenMixin`, `CitizenState`, `CitizenKind`, `SpargusDefense`, `JakCitizenRenderer`. Preservan Villager/brain/camas/POI/comercio Minecraft. Aprovechar metadatos y cues existentes, evitando duplicar los de defensa.
- Leer [docs/PORTING-DELTA.md](docs/PORTING-DELTA.md), [docs/PORTING-SCOPE.md](docs/PORTING-SCOPE.md), [docs/INVENTORY-ECONOMY.md](docs/INVENTORY-ECONOMY.md) para la diferencia original y la adaptación. El handoff histórico [docs/CURRENT-PORT-HANDOFF.md](docs/CURRENT-PORT-HANDOFF.md) conserva contexto y fallos anteriores; su historial no sustituye el estado más reciente de este documento.
- [MODLOG.md](MODLOG.md) tiene el historial de fallos/recibos, no solo resultados favorables. [CONTROLES.md](CONTROLES.md) describe los controles actuales.

## 7. Última consulta: 32 chunks y más de 120 FPS

El usuario pidió una configuración para mucha distancia, preferiblemente 32 chunks y más de 120 FPS. Se consultó hardware y configuración, **sin aplicar cambios desde el agente**.

Hardware leído: **i7-8700K**, **RTX 2080 Ti**, **32 GB RAM**.

En la primera lectura había render 24, simulación 16, VSync sí y tope 120 FPS. Se recomendó render 32, simulación 6, tope 144/165, VSync no, empezar a 1080p, gráficos rápidos sin shaders, nubes no, mezcla 3×3, entidades 75 %, mipmaps 4 y heap 6 GB.

**Lectura real al preparar este archivo**, posterior a esa conversación:

| Opción actual | Valor |
|---|---|
| `renderDistance` | `32` |
| `simulationDistance` | `6` |
| `maxFps` | `140` |
| `enableVsync` | `false` |
| `biomeBlendRadius` | `2` (radio 2, mezcla 5×5; no 3×3) |
| `entityDistanceScaling` | `1.0` |
| Heap del perfil del launcher | **`-Xmx4G`**, no 6 GB |

No se sabe por esta lectura quién cambió las opciones. No escribir que el agente las aplicó ni que ya se comprobó el objetivo. **No hay benchmark de 32 chunks a >120 FPS.** La sospecha de límite por CPU/generación al volar rápido es una hipótesis, no una medición.

Mods actualmente instalados en esa instancia: Fabric API y JakMod. **Sodium y Lithium no están instalados.** Sus páginas oficiales de Modrinth anuncian compatibilidad 26.3; fueron propuestos como candidatos, **sin probarlos con JakMod ni instalarlos**:

- https://modrinth.com/mod/sodium
- https://modrinth.com/mod/lithium

Si se continúa con rendimiento: conservar un baseline, probar compatibilidad y medir una ruta repetible con mundo precargado y otra de generación nueva/varios biomas en Hellcat. Separar FPS/frame pacing de ticks del servidor y generación de chunks. No esconder efectos/contenido de Jak para anunciar una cifra. No resucitar lowpoly propio como atajo.

## 8. Forma de continuar y comandos

Durante esta preparación no se encontró cliente Java/Javaw de esta instancia ni cliente dev del proyecto abierto. **Volver a comprobar al retomar**, incluyendo `java.exe` y `javaw.exe`; no asumir que el usuario sigue con el juego cerrado. No actualizar el JAR de una partida abierta. No matar procesos Java globalmente ni tocar otras aplicaciones/proyectos.

`tools/run_scene.py` usa copias privadas de `run/saves/New World (1)`, espera al backend SDL/primer render cuando corresponde, genera recibos y cierra su cliente. No usar una partida personal como arena destructiva. Sus escenas pequeñas prueban límites concretos: no presentarlas como prueba de generación normal, mundo completo o FPS. Para generación normal usar los fixtures específicos existentes.

PowerShell, desde la raíz del repo:

```powershell
$env:JAVA_HOME = "$env:LOCALAPPDATA/Packages/Microsoft.4297127D64EC6_8wekyb3d8bbwe/LocalCache/Local/runtime/java-runtime-epsilon/windows-x64/java-runtime-epsilon"

# Compilar Java sin volver a copiar 13.000 recursos en cada edición pequeña:
./gradlew.bat compileClientJava -x processResources -x buildNative --no-daemon --console=plain

# Tras cambiar el núcleo/ABI, antes de cualquier prueba real:
cargo test --workspace --manifest-path native/Cargo.toml
./gradlew.bat build check distributionJar --no-daemon --console=plain

# Ejemplo: solo si el cambio requiere contrastar ese caso:
python tools/run_scene.py --scene integration

# Distribución: reconstruir después de cerrar/verificar el cambio:
python tools/build_installer.py
python tools/audit_distribution.py --package dist/JakMod-Installer
python installer/test_install.py
```

El runner realiza su prebuild antes del plazo de arranque. No ejecutar builds, instalaciones y pruebas que muten los mismos binarios en paralelo. Antes de publicar, excluir `src/generated`, `.local`, `run`, saves, GLB/OGG/GO/ISO y JAR fusionado local. Tras instalar, comprobar recibo fresco y entradas del payload, no solo existencia del archivo o salida del EXE GUI. La creación de este handoff no exige nuevas pruebas del juego.

## 9. Sesión Claude 05/11 tarde: control total, port desde el Rust (sin HUD)

Adrii dio control total del proyecto y pidió seguir trayendo cosas del Rust **salvo el HUD** (la mayor parte existía solo porque el HUD de MW2 era malo). Referencia de faltantes: `Desktop/Repo/AUDITORIA-JAK-JAVA-2026-11-05.md` §5 y §10. Todos los mundos de la instancia y de `run/saves` son de prueba: crear mundos nuevos sin miedo (`--fresh-world`). Nada commiteado ni subido.

Cambios (código y datos):
- **Decisiones de Adrii restauradas**: vuelo de Jak luminoso gratis (`native/jak-core/src/lightjak.rs` = copia exacta del Rust); golpes cuerpo a cuerpo no dañan al HellCat ni a zoomers/coches: el primero apaga el HellCat encendido, tres en 1,6 s lo convierten en su objeto conservando casco/munición (`JakHellcat.strike`, bit 8/16 en `hellcat_host.rs`); creativo ya no da munición infinita (`JakPlayers` sin flag 32); receta de la JET-Board.
- **Recetas del Rust** (`data/jakmc/recipe`): tabla, HellCat, Hell Dog, 8 vehículos, 12 armas en cadena (Wave Concussor insertado en el orden rojo de Jak 3: adaptación), munición desde minerales/pólvora, eco oscuro, bloques del Páramo. Los bloques del catálogo viven en el espacio **`jak:`**, los objetos del mod en `jakmc:`; una receta con id equivocado rompe la carga de todo el paquete de datos (pantalla «errores en los paquetes de datos»).
- **Capa de flota** (`entity/VehicleItem.java`): objetos de vehículo con barra de casco; eco verde soltado encima en el inventario repara 25 % por eco; disparos de Jak quitan 2 PV a zoomers/coches; zoomer hundido → su objeto. Tráfico no se porta: el Rust ya lo había quitado.
- **Audio**: mezcla del Rust horneada en los OGG (`tools/import_audio.py` `mix_gain`: Jak 1 ×1,6, Jak 2/3 ×4,5, pasos ×18, limitador) porque Minecraft limita el volumen a 1; `bike-hop` de Jak 2 CTYWIDE2 importado; pasos vanilla silenciados bajo Jak (`JakStepSoundMixin`). Wave Concussor ya sonaba (`red2-shot` en la explosión).
- **Robustez**: teletransporte y cambio de dimensión conservan la misma sesión nativa (forma, eco, arma, tabla) con `Session.carry`; muerte/respawn, montura o HellCat siguen recreando. Locks nativos de vehículos/FX se recuperan si se envenenan. Error bilingüe claro si no carga el núcleo/exportaciones.
- **Tierras Jak** (`worldgen/JakLands.java` + `JakLandSurfaceMixin`/`JakLandDecorationMixin`), port de `jak_lands.rs`/`wasteland.rs`/`snowy.rs`/`swamp.rs`: desiertos = Páramo (arena, roca, acantilados, bordes, matorral, octavos de duna, cuevas, hierba seca, guijarros, palmeras junto al agua, agujas y mesetas rojas), nevados = Montaña Nevada (nieve en octavos, roca, pinos/postes, carámbanos), pantanos = Boggy Swamp (barro, corteza, musgo, nenúfares, espuma). Aldeas y pirámides vanilla del desierto suprimidas por etiquetas. Ojo: en 26.3 `ChunkAccess.getHeight` ya es el bloque superior (no el primer aire); la densidad debe leerse de `noise.cachingSamplers()`.

Evidencia (`.local/evidence/`): `rules-*` último **passed** (12 recetas, HellCat/zoomer por golpes, casco 13/15 tras un disparo, hundido, eco verde, misma sesión tras TP/Nether/vuelta, 0 pasos vanilla y 11 de Jak); `lands-*` último **passed** en mundo nuevo (Páramo 24.674 bloques Jak/0 vanilla, Nieve 27.621/0, Pantano 14.621/2.194 por estar en el borde), capturas `lands-*.png` revisadas. `cargo test` 94+7, `build check distributionJar` correcto. No reinstalado en la instancia del launcher ni empaquetado instalador.

Siguiente según la auditoría (sin HUD): nidos/bandas/reposición y merodeadores, ragdoll Grunt de 47 nodos, comportamiento de ciudadanos, explosión con restos de vehículos, cámara con desvanecido de paredes y rescate, raíles colocables y trucos L2 de tabla, cielo/lluvia Jak por tierra, eco oscuro líquido, fachadas/objetos sueltos.

Instalado el 05/11 a las 15:34 CET en la instancia `jakmod` (perfil `26.3.jakmod`) con el instalador reconstruido desde el árbol de trabajo: recibo nuevo `jakmc/setup/installation-receipt.json`, JAR SHA-256 `7a766dfac5e20ae348689cb963ea244a0ef02475dd6ca17b1de5fe9e27cdf9a9`, 15.938 recursos, 2.818 clips con mezcla. El JAR instalado contiene JakLands, VehicleItem, JakStepSoundMixin y las recetas nuevas. Auditoría de distribución sin fallos. RAR/release de GitHub no actualizados.
