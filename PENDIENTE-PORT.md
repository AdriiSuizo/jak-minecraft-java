# Lo que falta portear del Rust (lista viva)

Fuentes: `Desktop\Repo\AUDITORIA-JAK-JAVA-2026-11-05.md` (§5.1, §10) y el registro final del Rust
(`Documents\Codex\2026-10-25\contin-a-el-trabajo-jak-minecraft\outputs\CONTINUACION-JAK.txt`).
Regla: cada cosa se portea de su función en el Rust (y este del OpenGOAL), sin inventar datos.
Lo que sea un sustituto se marca en el código con `stand-in:` diciendo qué hace el original.
El HUD queda fuera salvo que se pida. Una partida de prueba por lote, no por línea.

Estados: HECHO · EN CURSO · PENDIENTE · FUERA (HUD/no pedido)

## Hecho (05/11)
- HECHO Vuelo de Jak luminoso gratis; creativo sin munición infinita; receta JET-Board y recetas Rust.
- HECHO Golpes → grietas → vehículo como objeto con casco; eco verde repara; zoomer hundido vuelve como objeto.
- HECHO HellCat: un golpe lo apaga, tres lo desmontan.
- HECHO Mezcla de volumen del Rust y pasos vanilla silenciados bajo Jak.
- HECHO Sesión nativa conservada al teletransportar y cambiar de dimensión; mutex de vehículos recuperable; aviso si falta la DLL.
- HECHO Biomas Jak: Páramo, Montaña Nevada, Pantano (terreno, octavos, agujas, mesas, palmeras); sin aldeas ni pirámides vanilla en desiertos.
- HECHO Puñetazos y patadas rompen bloques con la fórmula del Rust (daño ×100/8 ÷ 160 ÷ dureza); el bloque es el que toca la esfera de golpe del núcleo.
- HECHO FPS: el mando cuenta como actividad (Minecraft bajaba a 30 FPS al minuto y a 10 a los 10 min); el modelo de Jak ya no genera ~40 MB/s de basura.

## Hecho en el lote de la noche (05/11, instalado 19:22; compila; probado en partida solo lo marcado ✔)
- HECHO ✔ Puñetazos: daño a todos los bloques que toca la esfera de golpe del núcleo (6 de tierra con 13 golpes).
- HECHO ✔ Cámara: recorte redondo de paredes entre cámara y Jak (pilar comprobado); K rescate y desatasco (sin probar).
- HECHO ✔ Nidos y bandas por bioma (se vieron en el registro); reposición 240 s, merodeadores nocturnos, depredador junto a aldeas (sin probar).
- HECHO Cielo de Jak 1 / clima Jak 3 / relámpagos (carga las 32 partículas originales; revisar a ojo).
- HECHO Eco oscuro líquido, cubo, muerte por eco oscuro.
- HECHO Raíles de grind colocables (auto-unión de raíl vanilla); L2 de tabla y giro de suelo al núcleo.
- HECHO Ragdoll del Grunt al morir (47 articulaciones, 120 Hz).
- HECHO Ciudadanos: rutina, puertas y casas, poses, pánico, pasos, eco verde, tamaño aleatorio, campana, 5 habitantes de Jak 1, monjes.
- HECHO Explosión de vehículos con carcasa y restos; marchas del HellCat, roce con hojas, depósito de carbón.
- HECHO 19 fachadas, objetos sueltos, Spargus en desierto, Sandover de nieve/pantano, texturas por posición.

## Hecho en el lote de la tarde-noche (05/11, instalado 21:20; probado en partida lo marcado ✔)
- HECHO ✔ Agarre a bordes: muros de 2 y de 3 bloques. Causa: al teletransportarse, el núcleo volvía al cuerpo original de 0,7 m (demasiado gordo) y chocaba antes con el muro.
- HECHO ✔ Escalones: un bloque entero no se sube andando (se para delante); una losa sí.
- HECHO ✔ Puñetazos: primero el bloque marcado por Minecraft, después el de debajo (2 por muro).
- HECHO ✔ Sandover rojo vuelve a generar (llanuras y bosques), junto al nevado y al del pantano.
- HECHO ✔ Tráfico de zoomers en Haven, encendido por defecto (JAKMC_TRAFICO=0 lo apaga): solo por calles del plano libres en el mundo y con sitio para dos carriles; en cada cruce elige calle al azar, da la vuelta en los callejones, frena ante peatones.
- HECHO ✔ Conductores con el modelo original citizen-norm-rider (postura de moto o coche, se inclina con el giro), pegados al movimiento suavizado del zoomer (antes iban en pose T y a 20 Hz).
- HECHO Nombres en español de ~1.200 bloques y objetos (tabla tools/nombres_es.py que usa el generador). Los nombres de armas son traducción propia, no los oficiales.
- HECHO La tabla flota sobre el eco oscuro como sobre el agua (los zoomers ya flotaban sobre cualquier líquido).
- HECHO JakLands ya no pisa cofres ni otros bloques con entidad de las estructuras.

## Hecho 05/11 21:38 (instalado)
- HECHO ✔ Grunt: un golpe que no le mata lo tira como muñeco de trapo (la misma caída de 47 articulaciones que al morir); otro golpe en el aire lo empuja más; cuando se para se levanta con grunt-getup-front o -back según cómo haya caído; si muere estando en el suelo, su caída de muerte sigue desde ahí.

## Hecho 05/11 22:01 (instalado)
- HECHO ✔ Rayo: su destello ilumina el mundo (luz del cielo + 0,6 × destello, como minecraft_world.rs); respeta la opción de Minecraft de ocultar destellos.
- HECHO ✔ Niebla como el Rust: color de la niebla de Jak por hora y clima en lugar del de Minecraft, y la distancia acortada por el clima (fog_distances). Quitada la mezcla de inicio de tormenta de arena que el Rust no tenía.
- HECHO ✔ Gradación de color de Jak Enhanced Trilogy (licencia ISC en third_party/jak-enhanced-trilogy): campo, bosque, desierto, pantano y nieve, de día y de noche, mezcla de 1,5 s. IW4L_ENHANCED_GRADE=0 la apaga. Es sutil, como en el Rust.

## Hecho 05/11 22:20 (instalado)
- HECHO ✔ Población de ciudades como el Rust: al acercarse Jak (150 bloques) el barrio de Haven se llena hasta 28–44 ciudadanos (salió 40) y Spargus hasta 6 (como mucho una persona por cama), repartidos por sus calles; al volver tras alejarse más de 220 se completa otra vez. Ya no salen residentes al generar el mundo en esas dos ciudades. Las calles con gente hacen frenar al tráfico.
- HECHO ✔ Spargus repone sus 2 leapers sueltos.
- HECHO ✔ Alarma: el segundo golpe de Jak al mismo ciudadano (no a los de Spargus) pone a los guardias contra él 30 s, más 3 s que esperan mientras alguno le persigue; luego se calman. Sustituye a la regla anterior de "dos golpes vistos por un guardia".
- Sin comprobar en partida: que los guardias de verdad vayan a por Jak durante la alarma (en la prueba Jak volaba alto y no le veían).

## Hecho 05/11 22:45 (instalado)
- HECHO ✔ Carcasa del HellCat como el Rust: destellos amarillo/naranja 0,6 s y después oscurecida a un cuarto.
- HECHO ✔ El HellCat al explotar tira a Jak (dentro, subiendo o a menos de 3 m): un metro sobre el asiento, a media velocidad del HellCat y 12 m/s hacia arriba.
- HECHO ✔ Carcasas de zoomers y coches oscurecidas a un cuarto (antes sin luz) y trozos negros; el Saltadunas y el Arenatiburón conservan sus ruedas en la carcasa.
- HECHO ✔ Morir en eco oscuro lanza la ráfaga original group-dark-eco-death de Jak 1 (además del sonido que ya había).
- Escena nueva de prueba: tools/run_scene.py --scene wrecks.

## Hecho 06/11 00:16 (instalado)
- HECHO ✔ L2 con un arma en la mano solo apunta: ya no abre puertas ni monta en vehículos (Adrii 05/11). Sin arma, L2 sigue siendo el "usar" de Minecraft.
- HECHO ✔ Jak Claro, parar el tiempo como en Jak 3 (`target-lightjak-freeze`): ahora es un estado propio con la palmada (jakb-lightjak-freeze, del fotograma 0 o del 9 si ya llevaba 2 s como Jak Claro, hasta el 20). Al chocar las manos: destello de la mano izquierda, vibración del mando 0,5 s y sacudida de cámara (smush 819.2/15/75/0.9). El mundo empieza a frenarse en la palmada, no al pedirla. Si se pide antes de transformarse, la palmada llega tras el get-on.
- HECHO ✔ Pantalla con el tiempo parado: filtro azul (1, 1, 1,5) y la mezcla del fotograma anterior con estelas hacia los bordes, sacados de OpenGOAL (blit-displays.gc y los shaders slow_time, licencia ISC en third_party/opengoal).
- HECHO ✔ Escudo como en Jak 3 (`target-lightjak-shield`): un estado propio. Al subir el escudo, Jak gira 7,5°, da un saltito (8,25 m/s), hace la patada con su desplazamiento y el sonido SPIN_KICK, y vuelve a estar de pie o cayendo. Patea una vez por cada escudo, al entrar a estar de pie, saltar, caer o aterrizar.
- HECHO ✔ Los poderes de Jak Claro ya no se pisan: volar es su propio estado (las alas no se pierden por usar otro poder después) y la palmada y la patada son estados aparte, como la curación. Lo único que se excluye es lo que excluye el original: el escudo no sube mientras el tiempo está parado ni en los 0,5 s después de curar.
- HECHO ✔ Tirón al sacar el escudo por primera vez: la textura animada del escudo y de las alas (de 0,3 s) se prepara al cargar, fuera del hilo del juego. Medido: el fotograma de 344 ms ya no aparece; con el escudo abierto, 13,4 ms de mediana frente a 13,1 sin él.
- Arreglado en las escenas: munición en el mundo nuevo (escena de poderes), un bloque picado por pulsación de R2, el paso de sacar la tabla con Triángulo (escena de mando).
- Avisos de licencia ISC en el instalador: OpenGOAL y Jak Enhanced Trilogy (licenses/).

## Hecho 06/11 00:59 (instalado)
- HECHO ✔ Tráfico de Haven por una calle de ronda (Adrii 06/11): los zoomers dan vueltas en un solo sentido por el borde del barrio, 16 calles, sin callejones ni vueltas en U y sin tocar el palacio. Si alguna calle del borde está tapada en el mundo, vuelve a la red de antes.
- HECHO ✔ Los ciudadanos se apartan de la calzada cuando viene un coche, como el aviso `'clear-path` del tráfico de Jak 2: se van al hueco libre más alejado a su lado, hasta 10 m. Ya no salen ciudadanos nuevos en la ronda ni en sus cruces. En la prueba, los coches iban a unos 8 m/s sin nadie delante.

## Hecho 06/11 (lote 2–11; probado en partida lo marcado ✔, una vez por escena)
- HECHO ✔ Rana del pantano con lengua eléctrica (punto 11). La rana con lengua de los originales es `kermit`, de la ciénaga de Jak 1 (levels/swamp/kermit.gc); el "hopper" de la selva no tiene lengua. Portada función por función: salta para quedarse a 10 m de su presa; si la tiene a menos de 12 m y con la cabeza a menos de 20°, saca la lengua (kermit-lash; en el fotograma 14 decide si acierta; si falla se queda en 0,8 y la recoge). Si acierta, la lengua se queda pegada y un pulso eléctrico la recorre a 0,3 por segundo (partículas originales group-kermit-pulse y su impacto, sonidos kermit-loop, kermit-stretch y get-shocked). Al tocar a la presa le hace 2 de daño. Se suelta si la presa se aleja más de 18 m, si algo tapa la lengua o a los 4 s. Ataca a Jak y también a mobs y animales. A Jak la lengua le tuerce la dirección del stick hacia la rana (target-apply-tongue: 0,15 a 5 m, 0,22 a 15 m). Como en el original, si Jak está quieto no lo arrastra. Vive en pantanos (una banda propia de dos ranas) y tiene huevo "Generar Rana del pantano". En la prueba: a Jak y a una vaca, lengua pegada, pulso visto y 2 de daño a cada uno.
  - Sustitutos: no hay malla de navegación (se usa la línea de visión y la colisión de Minecraft); el cuello no gira por separado (manda el cuerpo); a los mobs no les tuerce la marcha; con la lengua pegada Jak puede seguir atacando.
- HECHO ✔ Tirón de Jak Claro la primera vez (punto 2): en los tres primeros fotogramas se dibujan, diminutos, el cuerpo luminoso, el oscuro, las alas y el escudo para que la tarjeta gráfica los prepare antes. Medido: el peor fotograma del escudo pasa de 111 a 50 ms y desaparecen los de 69–111 ms.
- HECHO  Nidos de la cueva del cráter (punto 5): la marca CAVE_MARK de la cueva crea un nido con su banda de cueva. Los enemigos invocados con comando o huevo pasan a ser merodeadores nocturnos, como en el Rust. Sustituto: la banda de cueva no mira la luz (el Rust la ponía con faroles y lava, y por eso nunca salía).
- HECHO  Jak oscuro (punto 6): las víctimas de sus rayos se tiñen de morado mientras les dura, y la Fuerza suma +3 por nivel a los golpes (Debilidad resta 4 por nivel, sin bajar de 0).
- HECHO ✔ Super Nova como bomba (punto 6b): explosión de potencia 16 que rompe el terreno como un TNT enorme, daña y empuja a todo menos al que dispara y su vehículo. Se ve la seta original (mushroom de Jak 3, animada 4 s), destello blanco 2 s, vibración y sacudida de cámara al segundo.
- HECHO  Flut Flut salvaje (punto 7): si lo golpean entra en pánico 5 s, sale despedido y huye zigzagueando; tras un intento de doma fallido, 3 s de pánico. Suelta 1–3 de pollo crudo, 0–2 plumas y la silla. Se puede atar con correa. Las crías siguen al adulto más cercano.
- HECHO Parpadeo de Jak y Daxter (punto 8), con la tabla original de eye.gc (cada 60–240 fotogramas).
- HECHO Del registro final del Rust (punto 9): succión de gemas con su rampa original y bloqueada al colector, guardada en el mundo; caída lenta: Jak cae como mucho a 4 bloques por segundo (no con el Flut Flut). La barra del Hip Hog no hacía falta (sus 103 celdas ya tienen suelo).
- HECHO Lote de la comparación (punto 10):
  - Lava: Jak nada en ella y arde (fuego ≥ 15 s pisándola, 1 de daño por segundo dentro); el escudo apaga el fuego e impide el daño de lava y fuego.
  - Hambre: correr 0,05 por bloque, saltar 0,05, golpe que acierta 0,1.
  - Patadas: abren puertas, trampillas y pulsan botones; una patada toca la campana (un golpe de suelo la rompe).
  - Inversor de masa sobre mobs: los deja flotando, al golpearlos dan vueltas, caen como hayan quedado, se quedan tumbados y se levantan despacio (tumble.rs).
  - HellCat sobre agua, lava y eco oscuro: el líquido es un suelo plano para él y sale hacia arriba si se hunde.
  - Invisibilidad: oculta a Jak y a Daxter y los enemigos lo ven solo de cerca. Salto potenciado: más salto de Jak.
  - Primera persona: balanceo del arma al andar y retroceso al disparar.
  - (Quitado el 06/11: el saltador del skatepark, nadie lo pidió; en su lugar el lanzador de eco azul.)
  - Sin probar en partida: lava, hambre, patadas, HellCat en líquidos, invisibilidad y balanceo (compilan y siguen el original, pero no hay escena para ellos).

- HECHO ✔ Con arma en las manos, la tabla a la espalda va con el torso (Adrii 06/11): la tabla cuelga de la cintura y se quedaba con la animación de andar mientras el torso hacía la de arma; ahora toma también la de arma, como en el original. Visto en las capturas de la prueba de armas.
- HECHO Daxter con arma (Adrii 06/11: su brazo derecho atravesaba el pelo y la frente de Jak): Daxter seguía con su animación normal mientras Jak hacía la de arma. Ahora hace la suya del original (daxter-gun-stance, -walk, -fire, -takeout…), como en sidekick.gc. Visto solo desde detrás en la prueba; falta mirarlo de frente.
- HECHO ✔ El Inversor de masa deja sin gravedad a todo, también a los objetos (Adrii 06/11): objetos tirados, flechas, barcas, vagonetas, bloques que caen, TNT y orbes de experiencia flotan igual que las criaturas (suben a 1 m del suelo, se quedan entre 1 y 3,5 m, 7–9 s, y luego caen). El campo crece hasta 30 m en su primer segundo, como el original. Los jugadores no. En la prueba de anillos atrapó 22 objetos; unas piedras, una barca y una vagoneta subieron 1 bloque.
- Arreglado en la prueba de armas: un mundo nuevo empieza sin munición y las armas no disparaban; ahora carga munición de cada color. Con eso, las 12 disparan otra vez (la Super Nova rompe el suelo).
- Sin comprobar en partida con medida propia: tinte de Jak oscuro y bono de Fuerza, nidos de la cueva, pánico del Flut Flut (sus escenas pasan, pero no miden eso).

## Lote de Adrii 06/11 (18 puntos + esclusa)
- HECHO Pasos sobre hojas de árbol suenan a hierba; agarre a bordes con la altura del Rust (1,9 bloques) y suelo bajo Jak buscado más abajo.
- HECHO Menú creativo: el stick derecho es la rueda del ratón esté donde esté el puntero.
- HECHO Decoración: mira hacia donde mira el jugador al ponerla; jarrón y palmera con caras por los dos lados; barriles opacos.
- HECHO Iconos: los objetos y bloques de Jak sin icono propio usan su modelo 3D (43 objetos, 1.223 bloques).
- HECHO Vista previa al 18 % de los objetos de más de un bloque (también las puertas 3x3).
- HECHO Alcance para poner bloques ×3; el de romper y atacar, igual.
- HECHO Fuego del bidón más grande y más bajo (escala de bloque del Rust).
- HECHO Quitado el saltador del skatepark. Lanzador de eco azul de Jak 1 (citb-launcher): solo con eco azul; Jak se agacha, sale con el salto de lanzador de Jak 3 (target-launch → duck-high-jump 'launch, 40 m de muelle, launch-jump-mods).
- HECHO Puerta iris de la Roca del Géiser, 3x3, solo las aspas: se abre con eco azul cerca (8 m) y se cierra al alejarse (stand-in: la original se queda abierta).
- HECHO Esclusa de Villarefugio (com-airlock-outer y -inner espalda con espalda), 3x3: se abre a quien se acerque y se cierra al irse; cada cara con su animación de airlock.gc y sus sonidos de CTYWIDE1, de un bloque de grosor y acelerada para que abrir o cerrar dure como mucho 2,5 s (Adrii; stand-in: 8/12 m en vez de 35 m; no carga zonas).
- HECHO JET-Board: el agua y el eco oscuro son suelo para la tabla (la lava no) y los lados de bloque cuentan como suelo para subir; la tabla a la espalda ya no se deforma (se copiaban las poses guardadas).
- HECHO Jak luminoso con su propio pase opaco, sin transparencias (mismo coste que antes).
- HECHO Villarefugio: hasta 48 guardias por ciudad en sus puestos, reaparecen de día; atacan a los mobs hostiles. Cada ciudadano toma una cama libre de la ciudad.
- HECHO Tráfico: los zoomers con ruta no dan marcha atrás; si se atascan 2 s pasan del obstáculo que no es jugador; choques entre zoomers; vaivén suave del Rust.
- HECHO Jak oscuro: los golpes normales no gastan eco; al volver a Jak con menos de un punto queda a 0 (antes quedaba un resto invisible); el HUD redondea hacia arriba.
- HECHO Bomba oscura (X+Cuadrado, target-darkjak-bomb0): con 5 de eco oscuro o más (el 50 del original, a la décima como los golpes); sube, cae con gravedad extra y al tocar suelo explota en una esfera que crece hasta 30 m, 15 de daño; gasta 10 (100 en el original). Explosión oscura (X+Círculo, o agachado+Círculo como el original; target-darkjak-bomb1): sube, flota 1,5 s y lanza un rayo cada 0,1 s a cada criatura en 80 m, 15 de daño y la congela (congelación de Minecraft); gasta 10. Las dos dejan a Jak oscuro (Adrii; el original vuelve a Jak). La bomba dibuja su cúpula (bomb-blast y generic-blast de Jak 3: blanco, morado, negro en 30 cuadros).

- HECHO Todo lo decorativo se fabrica: 1.288 piezas de Jak (vallas, tejados, escaleras, objetos sueltos…) en el cortapiedras desde un bloque del material de su lugar (arenisca en Spargus, ladrillo de piedra en Villarefugio, cuarzo en el palacio…) o de lo que es (madera, hierro, cristal, tela, hojas). Lanzador y puertas en la mesa de crafteo.
- HECHO Desierto de Jak: sus texturas (arena, roca, acantilados, cuevas, hierba seca, piedras, palmeras, roca roja, tótem) no estaban en los niveles que lee el exportador y salían rayas de relleno; ahora son las de las páginas de texturas de Jak 3. La arena, a sus 128 texeles, se extiende sobre 4x4 bloques sin girar, como en Jak 3. Siguen de relleno vil1-medres-hutroof y snow-gate-wood (no están en ninguna página).
- HECHO Tests: cada uno crea un mundo nuevo y borra todos los anteriores (los ajustes del mundo quedan en .local/test-world-template).

## Pendiente, por orden
- Nada del Rust en la lista. Lo siguiente lo decide Adrii.

## Fuera por ahora
- Golpes de mobs que llegan 0,45 s después del gesto (Adrii 06/11: quitado de la lista).
- Fachadas: etiquetas de pico/hacha y regla por cabaña de Sandover; destello de la Super Nova (Adrii 06/11: quitados de la lista).
- FUERA (Adrii 05/11) Creativo como el Rust; triángulo "lo usable más cercano"; kit inicial; desgaste de armas; portal al skatepark de Jak 2 (ya hay parte del skatepark en el centro de la torre de Villarefugio, con la tabla del altar).
- FUERA HUD de vuelo, iconos de armas, timer de Jak 1, anillo animado, sprites de botón (HUD).
- FUERA Menús UI42–UI69 1:1 del Rust (interfaz; preguntar si se quieren).
- FUERA Edit Blaster (sustituido por el Wave Concussor).
