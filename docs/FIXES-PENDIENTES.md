# Fixes y diferencias pendientes del port de Jak

Inventario del 4 de noviembre de 2026: **33 trabajos abiertos**, **11 contrastes concretos** y **3 pendientes que ya existían en el origen**. Incluye los fallos actuales del usuario y las peticiones nuevas de controles/explosión.

El inventario inicial no cambiaba el juego. El avance siguiente registra las correcciones posteriores sin cerrar variantes todavía sin contrastar. Se parte de PORTING-DELTA, del handoff y de los puntos de entrada/salida del código. Las pruebas antiguas de poses/efectos y daño a entidades no cubrían la destrucción del mundo. El arranque corregido del launcher tampoco certifica paridad jugable completa.

**Estados:** «Ausencia confirmada» = falta una ruta en código; «Reportado» = fallo observado por el usuario; «Parcial» = hay integración, falta un comportamiento concreto; «Petición» = cambio solicitado; V = comparación específica pendiente, no un fallo demostrado.

## Avance del 5 de noviembre

La tabla se saca/guarda con Triángulo/Y solo cuando está equipada en su ranura; L2 usa Minecraft y R2 ataca/mina o dispara Morph Gun. Se conserva el teclado B y los gatillos de conducción. La prueba SDL `input_actions-20261105-000321` comprueba esas condiciones y Select/R3.

Correcciones implementadas: salto desde agua sin descontar dos veces la separación de suelo; volumen conjunto de montura y jinete y movimiento con colisiones; puente de daño acumulado a bloques para Strike/Impact/WorldHit; despacho de las cuatro explosiones originales; daño de terreno para Hellcat/Helldog y coches; onda más pequeña, con obstrucción por paredes y empuje. La toma `integration-20261105-002439` comprueba salto simple de 0,450154 bloques sobre agua, contacto frontal del Flut Flut sin penetración ni daño, disparos nativos que rompen un muro, vegetación presente antes de explotar, drops, bedrock y ausencia de daño duplicado. Las otras direcciones, orillas y variantes siguen pendientes; los estados por ID se mantienen en `pendientes.json`.

## Orden de trabajo

Agua/colisiones y daño al mundo, después interacción de objetos y las restantes integraciones. El usuario ha retirado por completo el trabajo de lowpoly/lejanos de esta versión Java. No modificar los repositorios originales, OpenGOAL ni partidas personales para validar estas correcciones.

## Agua y colisiones

### F01. Salida del agua al saltar

**Estado:** Reportado.

El salto desde natación apenas permite salir del agua. swim.rs coincide con el mashup; revisar el puente de altura de agua, escala y transición a salto.

**Código/oráculo:** native/jak-core/src/swim.rs; native/jak-jni/src/lib.rs; src/main/java/dev/jakmc/player/JakPlayers.java.

**Cierre:** Salir del agua y alcanzar una orilla normal con el impulso/animación de Jak, sin reentrada prematura en natación ni teletransporte.

### F02. Flut Flut: penetración de paredes

**Estado:** Reportado + puente parcial.

La entidad tiene volumen Minecraft, pero montada recibe setPos desde la simulación de Jak. El volumen del pájaro y el del jinete deben concordar con el barrido nativo.

**Código/oráculo:** src/main/java/dev/jakmc/entity/JakFlutFlut.java; src/main/java/dev/jakmc/entity/JakLeaper.java; src/main/java/dev/jakmc/player/JakPlayers.java:updateMount.

**Cierre:** Paredes, esquinas, techos y pasos estrechos detienen el conjunto por su volumen real.

### F03. Flut Flut: daño al rozar una pared

**Estado:** Reportado.

La penetración causa daño y puede realimentar la reacción de montura. Se corrige el contacto; no se sustituye por invulnerabilidad.

**Código/oráculo:** src/main/java/dev/jakmc/entity/JakLeaper.java:hurtServer; native/jak-core/src/flut.rs.

**Cierre:** Acercarse o caminar junto a una pared no resta vida ni activa una reacción de golpe falsa.

### F04. Leaper: mismo puente de colisión de montura

**Estado:** Parcial.

Comparte la actualización de posición de la montura con Flut Flut; la corrección debe cubrir también al lagarto.

**Código/oráculo:** src/main/java/dev/jakmc/player/JakPlayers.java:updateMount; src/main/java/dev/jakmc/entity/JakLeaper.java.

**Cierre:** Colisión conjunta correcta del lagarto y jinete al montar, conducir, saltar y bajar.

### F05. Zoomers: casco y contacto con bloques

**Estado:** Reportado por familia.

Ajustar el volumen barrido, la rotación y el contacto del casco en sus seis variantes, conservando la simulación original.

**Código/oráculo:** src/main/java/dev/jakmc/entity/JakVehicle.java; native/jak-jni/src/hover_host.rs.

**Cierre:** No atravesar esquinas, suelo ni techo; mantener el paso suave de desniveles y ambas alturas de circulación.

### F06. Hellcat y Helldog: colisión del casco completo

**Estado:** Reportado por familia.

La cubierta transitable no certifica el volumen exterior de la nave al volar o girar.

**Código/oráculo:** src/main/java/dev/jakmc/entity/JakHellcat.java; src/main/java/dev/jakmc/entity/HellcatDeck.java; native/jak-jni/src/hellcat_host.rs.

**Cierre:** Casco y extremos paran ante bloques en cualquier orientación; la cubierta conserva sus colisiones y daños por etapas.

### F07. Saltadunas y Arenatiburón: ruedas, casco y esquinas

**Estado:** Reportado por familia.

Cubrir el vehículo completo, además de los contactos verticales de ruedas ya implementados.

**Código/oráculo:** src/main/java/dev/jakmc/entity/JakVehicle.java; native/jak-jni/src/wheeled_host.rs.

**Cierre:** No introducir carrocería/ruedas en bloques; subir un bloque y desniveles diagonales sin choque falso ni frenazo, con aterrizaje y suspensión conservados.

## Daño al mundo y explosiones

### F08. Golpes cuerpo a cuerpo contra bloques

**Estado:** Ausencia confirmada.

Los eventos strike sin actor solo consultan hitProjector; no hay una ruta general de daño al mundo.

**Código/oráculo:** src/main/java/dev/jakmc/player/JakPlayers.java:applyEvent.

**Cierre:** Puñetazo, giro y ataques aplicables afectan al bloque alcanzado con dureza, permisos, efectos y drops coherentes.

### F09. Impactos de armas contra bloques

**Estado:** Ausencia confirmada.

Los impactos sin actor solo tratan la propaganda. Completar también las rutas de armamento de vehículos que correspondan.

**Código/oráculo:** src/main/java/dev/jakmc/player/JakPlayers.java:applyEvent; src/main/java/dev/jakmc/entity/JakHellcat.java:applyEvent; src/main/java/dev/jakmc/entity/WheeledWeapons.java.

**Cierre:** Impactos reales sobre hierba, decoración y bloques; comportamiento según arma y material, con autoría del servidor y sin roturas duplicadas.

### F10. Explosiones del Morph Gun que ya rompían terreno

**Estado:** Regresión de port confirmada.

JNI sí emite explosion; Java no lo despacha. El host original aplica poderes TNT Red3=4, Dark1=3, Dark3=9 y Yellow3=1.5.

**Código/oráculo:** native/jak-jni/src/lib.rs:event; src/main/java/dev/jakmc/player/JakPlayers.java:applyEvent; origen crates/render_anim/src/jak.rs:3519.

**Cierre:** Recuperar destrucción y cráter de Super Nova y las otras tres variantes, usando resistencia/drops del anfitrión y sin sumar dos veces el daño a entidades.

### F11. Hellcat/Helldog: romper vegetación y terreno

**Estado:** Ausencia confirmada + petición actual.

blast y explode terminan en sonido en el adaptador Java; los impactos a actores siguen otra ruta.

**Código/oráculo:** src/main/java/dev/jakmc/entity/JakHellcat.java:applyEvent.

**Cierre:** La explosión afecta al mundo, incluida hierba, con resistencia y drops Minecraft; el daño a criaturas se conserva una sola vez.

### F12. Hellcat/Helldog: onda expansiva

**Estado:** Petición actual.

Completar la onda y su respuesta física/visual coordinadas; un sonido o daño directo a un actor no completan esta mecánica.

**Código/oráculo:** native/jak-core/src/hellcat.rs; native/jak-jni/src/hellcat_host.rs; src/main/java/dev/jakmc/entity/JakHellcat.java; src/client/java/dev/jakmc/client/VehicleEffects.java.

**Cierre:** Onda visible y empuje con obstrucción por el entorno, sin daño a través de paredes ni doble impulso.

### F13. Hellcat/Helldog: reducir radio

**Estado:** Petición actual.

Reducir de forma coherente el alcance de daño, empuje y presentación de la explosión.

**Código/oráculo:** native/jak-core/src/hellcat.rs; native/jak-jni/src/hellcat_host.rs; src/client/java/dev/jakmc/client/VehicleEffects.java.

**Cierre:** Radio más pequeño que el actual, común a ambos modelos y consistente con lo que se ve; no cambiar solo el tamaño de un sprite.

### F14. Destrucción completa de Zoomers y coches

**Estado:** Parcial confirmado.

JakVehicle.wreck expulsa pasajeros, reproduce un sonido y elimina la entidad. Falta el ciclo completo de destrucción del vehículo.

**Código/oráculo:** src/main/java/dev/jakmc/entity/JakVehicle.java:wreck; origen crates/render_anim/src/zoomers.rs y wheeled_cars.rs.

**Cierre:** Conectar los estados, restos, efectos y consecuencias originales aplicables, manteniendo persistencia y una única eliminación.

## Objetos y mando

### F15. L2: interacción y uso Minecraft

**Estado:** Cambio solicitado; asignación actual distinta.

L2 está usado como apuntado general; la interacción nativa usa Triangle y parte de R2.

**Código/oráculo:** src/client/java/dev/jakmc/client/JakInput.java:poll; src/client/java/dev/jakmc/client/input/JakMenuControls.java.

**Cierre:** L2 abre/usa bloques y entidades y la función secundaria del objeto; conserva el apuntado del Morph Gun cuando no corresponde una interacción.

### F16. R2: golpear, romper y acción primaria

**Estado:** Cambio solicitado; puente parcial.

La clasificación actual deja R2 sin ataque para varias categorías y lo dirige a BOARD por defecto. La colocación depende de apuntar.

**Código/oráculo:** src/client/java/dev/jakmc/client/JakInput.java:triggerFor y poll.

**Cierre:** Mano vacía, bloques, herramientas y armas responden a R2 con la acción apropiada; minado mantenido, golpes y uso no se pierden.

### F17. Prioridades entre objeto, tabla, montura, poderes y conducción

**Estado:** Parcial.

Evitar que una acción reserve el gatillo y anule las demás. El bloqueo busy actual incluye agua/montura.

**Código/oráculo:** src/client/java/dev/jakmc/client/JakInput.java.

**Cierre:** Usar puertas/objetos donde corresponde sin desmontar o desplegar la tabla por error. En conducción se conservan R2 acelerador y L2 freno; L3/poderes y zoom R3 mantienen su función.

### F18. Presentación del uso de objetos nativos en la mano de Jak

**Estado:** Parcial.

La comprobación de Morph Gun en primera persona no cubre todas las animaciones de uso de Minecraft en el esqueleto Jak.

**Código/oráculo:** src/client/java/dev/jakmc/client/model/JakRenderer.java; src/client/java/dev/jakmc/client/model/GltfItemRenderer.java.

**Cierre:** Comer, beber, usar herramientas y objetos cargables con acción real, posición/animación coherentes y sin fragmentos ni doble mano.

## Movimiento, combate y actores

### F19. Manos del agarre calibradas al cuerpo cinematográfico

**Estado:** Pendiente documentado.

El muestreador de boca del arma está cerrado; el feedback separado de las manos para los bordes conserva su pendiente.

**Código/oráculo:** native/jak-jni/src/body_joints.rs; native/jak-core/src/step.rs; docs/PORTING-DELTA.md.

**Cierre:** Las manos coinciden con el borde visible durante agarre, giro, subida y suelta, con distintas alturas y posturas.

### F20. Grunt: caída articulada original

**Estado:** Ausencia confirmada.

El original tiene solver de 47 articulaciones. El render Java aplica una inclinación global de caída a determinados clips, no ese solver.

**Código/oráculo:** origen crates/render_anim/src/grunt_ragdoll.rs; src/client/java/dev/jakmc/client/model/JakEnemyRenderer.java.

**Cierre:** Portar articulaciones, restricciones, contactos, golpes adicionales, recuperación frontal/trasera y muerte manteniendo la caída.

### F21. Reacciones completas a impactos por especie

**Estado:** Parcial documentado.

Hay estados de herido, caída y recuperación; quedan diferencias concretas por actor y ataques especiales.

**Código/oráculo:** src/main/java/dev/jakmc/entity/JakEnemy.java; src/main/java/dev/jakmc/entity/KrimsonGuard.java; docs/PORTING-DELTA.md.

**Cierre:** Cada golpe conecta una sola vez y muestra aturdimiento/recuperación de su especie; conservar animaciones y tiempos originales.

### F22. Volúmenes de golpe y drops durante caídas físicas

**Estado:** Parcial ligado a ragdoll.

El volumen y el punto de caída de la gema deben seguir al cuerpo/cabeza físicos; un actor inclinado visualmente no completa ese recorrido.

**Código/oráculo:** src/main/java/dev/jakmc/entity/JakEnemy.java:dropCustomDeathLoot; src/main/java/dev/jakmc/content/GemPhysics.java; origen grunt_ragdoll.rs.

**Cierre:** Acierto y gema en la posición real del cuerpo caído, sin zona golpeable de pie ni doble drop.

### F23. Multitudes y recuperación de rutas urbanas

**Estado:** Parcial documentado.

Existen patrullas y navegación Minecraft. Falta cerrar el comportamiento del conjunto en calles/puertas/vecinos frente al original.

**Código/oráculo:** src/main/java/dev/jakmc/entity/KrimsonGuard.java; src/main/java/dev/jakmc/worldgen/JakCityStructure.java; origen settlement_crowd.rs y creature_navigation.rs.

**Cierre:** Destinos estables, salida de atascos y tránsito entre vecinos; sin eses continuas ni alternancia de rutas imposibles.

### F24. Nidos, bandas y reposición de enemigos

**Estado:** Pendiente de integración.

El origen conserva nidos por hábitat, miembros de banda y tiempos de reposición; no basta el spawn nativo independiente.

**Código/oráculo:** origen crates/render_anim/src/jak_enemies.rs; src/main/java/dev/jakmc/entity/JakEnemy.java; docs/PORTING-DELTA.md.

**Cierre:** Recuperar las familias y reposiciones correspondientes sin multiplicar enemigos en interiores ni invadir otras dimensiones.

## Presentación y efectos

### F25. Primera persona de poderes/cuerpo fuera del Morph Gun

**Estado:** Pendiente documentado.

Los doce Morph Guns ya tienen su vista original; el resto conserva una entrada distinta en el inventario.

**Código/oráculo:** src/client/java/dev/jakmc/client/model/JakFirstPerson.java; src/client/java/dev/jakmc/client/model/JakRenderer.java.

**Cierre:** Poderes, escudo, alas y acciones aplicables con manos/cuerpo/efectos correctos en esa vista.

### F26. Distorsión y composición de efectos originales

**Estado:** Pendiente documentado.

Las bibliotecas originales de partículas están conectadas; quedan familias/pases como la distorsión de la imagen del anfitrión Rust.

**Código/oráculo:** origen crates/render_gpu/src/drawsurf/jak_distort.rs; src/client/java/dev/jakmc/client/JakEffects.java; docs/PORTING-DELTA.md.

**Cierre:** Integrar cada pase pendiente respetando orientación, alfa, profundidad, anclajes y coste; sin sustituirlo por un brillo genérico.

### F27. Voces y diálogos de habitantes

**Estado:** Pendiente documentado.

Los sonidos de pasos, golpes, armas y vehículos no completan las voces/diálogos de los actores.

**Código/oráculo:** docs/CURRENT-PORT-HANDOFF.md: native citizens; origen audio/src/jak.rs y jak2_villagers.rs.

**Cierre:** Conectar las voces disponibles y sus disparadores, sin voces duplicadas ni sonidos inventados.

### F28. Vibración original del mando

**Estado:** Ausencia confirmada.

El origen consume Event::Hurt.buzz mediante buzz_hurts. No aparece un consumidor de rumble/buzz en el adaptador SDL/JNI actual.

**Código/oráculo:** origen crates/render_anim/src/jak.rs:buzz_hurts; native/jak-jni/src/lib.rs:event; src/client/java/dev/jakmc/client/input/SdlGamepad.java.

**Cierre:** Recuperar los pulsos y tiempos del origen mediante SDL, vinculados al mando que controla al jugador.

## Ciudades, biomas y decoración

### F29. Catálogo de objetos todavía sin adaptación activa

**Estado:** Parcial medido.

114 definiciones de objeto; 31 IDs activos y 83 candidatos sin ruta activa de decoración/ensamblaje. Ver anexo.

**Código/oráculo:** src/main/java/dev/jakmc/content/JakProps.java; src/main/java/dev/jakmc/content/JakAssemblies.java; tools/import_content.py; docs/parity/catalogo-no-activo.json.

**Cierre:** Adaptar los detalles útiles a módulos sólidos y colocables. No reintroducir anillos/pasarelas descartados ni bloques casi vacíos para subir el contador.

### F30. Variedad procedural de barrios y edificios

**Estado:** Parcial confirmado.

CityBlueprint carga un plan por sitio; orientación, terreno y contenido variable no sustituyen las familias/layouts del generador original.

**Código/oráculo:** src/main/java/dev/jakmc/worldgen/JakCityStructure.java; src/main/java/dev/jakmc/worldgen/CityBlueprint.java; tools/content-exporter/src/main.rs.

**Cierre:** Generar variedad con el terreno y semilla conservando torre rodeada de ciudad, bar, garajes, interiores y circulación.

### F31. Biomas/regiones y vegetación de Jak

**Estado:** Parcial documentado.

Hay ciudades y sitios nativos y materiales regionales. Falta el cotejo/adaptación completa de Wasteland, nieve, pantano y su vegetación/encuentros.

**Código/oráculo:** docs/PORTING-DELTA.md; src/main/resources/data/jakmc/tags/worldgen/biome; origen jak_lands.rs, wasteland.rs, snowy.rs y swamp.rs.

**Cierre:** Mantener los rasgos originales en mundos normales, transiciones coherentes, Spargus solo en desiertos y población automática solo en Overworld.

### F32. Iluminación y materiales híbridos Jak/Minecraft

**Estado:** Parcial documentado.

Existen luz Minecraft y adaptación de materiales. Quedan diferencias de brillo, sombras, iluminación original y presentación uniforme entre actores y mundo.

**Código/oráculo:** docs/PORTING-DELTA.md; src/client/java/dev/jakmc/client/model/GltfModel.java; src/client/java/dev/jakmc/client/JakAtmosphere.java.

**Cierre:** Conservar oscuridad/spawns e iluminación decorativa real, sin niebla gris artificial sobre modelos ni transparencias de material incorrectas.

## Rendimiento

### F33. Objetivo de 150 FPS y estabilidad de exploración

**Estado:** Objetivo abierto.

El recorrido aceptado de siete biomas no acredita 150 FPS sostenidos. Sus percentiles y pausas están conservados en el handoff.

**Código/oráculo:** docs/CURRENT-PORT-HANDOFF.md: hellcat_crossing-20261104-170512.

**Cierre:** Optimizar sin recortar detalle/población/distancia ni ritmos de simulación; publicar p95/p99/picos y trayectos prolongados.

## Trabajo retirado por el usuario

F34–F36 (terreno lowpoly, torre lejana y mejora de modelos lejanos) se retiran por petición del 4 de noviembre de 2026. No se portarán a Minecraft Java. Los IDs históricos se conservan en `pendientes.json`, en `removed`, para no confundirlos con trabajo completado.

## Contrastes todavía abiertos, sin borrar correcciones anteriores

1. **V01 — Magnitudes físicas frente a escala visual.** Cotejar BLOCK=6400, jak.size=0.64 y dimensiones/canales del cuerpo; el origen asigna visual_scale. Relacionado con salto, agua y monturas, todavía sin atribuirle por sí solo los fallos reportados.
2. **V02 — Cadencia real de combos y autoapuntado.** Comparar entradas y respuesta con Jak, especialmente el tercer golpe oscuro, sobre objetivos y alturas distintos. Hay correcciones anteriores; no se etiquetan otra vez como ausentes.
3. **V03 — Bordes, escaleras y pasos diagonales.** Conservar los casos aprobados y cerrar alturas, esquinas, medio bloque y dirección diagonal del jugador, tabla y monturas.
4. **V04 — Tabla: secuencias sobre construcción del jugador.** Encadenar rail, salto, truco, rampa y aterrizaje, con geometría que cruza celdas. No sustituye el circuito ya probado.
5. **V05 — Reloj compartido en todas las variantes.** Hay correcciones de interpolación de cámara/cuerpo/mano/vehículo; falta contrastar cada variante y el movimiento rápido después de nuevos cambios de colisión.
6. **V06 — Cámara y oclusión.** Contrastar paredes, suelos y espacios cerrados sin cortes internos, corona de caras, saltos bruscos ni efecto sobre objetos de mano.
7. **V07 — Monjes y patrullas montadas.** Ya se generan monjes montados y se transfieren al robar la montura; comprobar el ciclo natural y sus animaciones/reacciones, no reimplementarlo como si faltara.
8. **V08 — Vivienda, profesión y comercio naturales.** Hay aldeanos/POI/comercios/camas y reposición nativa; cerrar el ciclo natural y la capacidad de viviendas en ciudades de distintas semillas.
9. **V09 — Piedras y eco en grupos y posiciones difíciles.** Las correcciones previas existen. Contrastar muchas unidades, mano, paredes, caída, atracción y drops de todas las especies, incluyendo la caída articulada pendiente.
10. **V10 — Efectos por familia, agua y clima.** Contrastar gotas/ripples horizontales, ataque de tabla, capas alfa y niebla/tormentas entre biomas; conservar las familias ya verificadas y reparar las diferencias concretas.
11. **V11 — Guardado y cambios de dimensión completos.** Se usan UUID/guardado nativo. Completar combinaciones de vehículos, monturas, objetos sueltos, inventario y reinicio real; evitar equiparar una caché de sesión con persistencia en disco.

## Pendientes heredados, no fixes perdidos por este port

1. **H01 — Eco amarillo de Jak 1 como proyectil.** La función existe sin consumidor también en el mashup. Es trabajo heredado pendiente, no una regresión nueva causada por el instalador.
2. **H02 — Gestión de misión del circuito, anillos, fuego y compuertas originales.** El cierre original ya la dejaba incompleta. El circuito físico, sus rampas/raíles/altar y el portón Krimson son elementos distintos.
3. **H03 — Atracción/recogida original completa de gemas.** El origen ya documentaba límites de atracción; la física adaptada y la recogida existentes no cierran todos los comportamientos del juego original.

## Catálogo: candidatos exactos

El exportador contiene 1.297 entradas de bloque, de las que 130 tienen ready=false. Ese total incluye rutas antiguas sustituidas por objetos/ensamblajes y no es una cuenta de 130 bloques perdidos. Hay 114 definiciones de objeto y 31 IDs activos mediante decoración/ensamblajes; los 83 restantes se enumeran en [catalogo-no-activo.json](parity/catalogo-no-activo.json). Su adaptación debe respetar la cuadrícula y excluir los anillos flotantes, pasarelas sin destino y geometría que el usuario pidió retirar.

## Correcciones existentes que se conservan

No vuelven a figurar como implementación ausente: Jak/Daxter cinematográficos y ropa; activación automática del launcher; ranura propia/durabilidad de tabla; inventario/creativo nativos; eco y munición de inventario/recetas/comercios; mantener Down para stack; L3 de poderes y zoom R3; turbo analógico/ilimitado y empuje aéreo; láser/balas y doce vistas de Morph Gun; bocas de armas animadas; puertas/altar/propaganda ya integrados; torre rodeada de ciudad, bar, garajes y Spargus en desiertos. Un fallo específico posterior reabre su incidencia concreta.

MW2, Skate 3, el motor Minecraft del anfitrión antiguo, los prototipos espaciales y una copia literal de mapas completos no se añaden como requisitos nuevos. Se reutilizan las mecánicas nativas de Minecraft donde cumplen el propósito.

## Referencias y mantenimiento

[Inventario de origen](PORTING-DELTA.md), [alcance](PORTING-SCOPE.md), [handoff](CURRENT-PORT-HANDOFF.md). Datos editables de esta lista: [pendientes.json](parity/pendientes.json). Mantener IDs y añadir evidencia al cerrar un punto; no sustituir una regresión por una etiqueta genérica «portado».
