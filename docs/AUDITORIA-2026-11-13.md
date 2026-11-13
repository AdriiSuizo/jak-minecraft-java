![Jak Worlds](media/jak-worlds-header.gif)

[Inicio](../README.md) · [Descargar](https://github.com/AdriiSuizo/jak-minecraft-java/releases/tag/v0.1.0-dev.19) · [Guías](README.md)

---

# Auditoría del mod y del instalador — 13 de noviembre de 2026

Alcance: Minecraft Java 26.3 en Windows x64, perfil normal. Comparación inicial `b4f8bca`; correcciones de funcionamiento en `659381e`; portabilidad de pruebas en `4d79153`. La variante Movie/Replay y los vídeos no forman parte de esta auditoría.

Se han corregido cuatro fallos funcionales demostrados. Cada cambio tiene una prueba que reproduce el fallo anterior y controles que preservan el comportamiento relacionado. Esto acredita los casos descritos, no una ausencia absoluta de fallos ni compatibilidad con cualquier combinación de mods.

## Fallos corregidos

| Caso reproducido | Corrección | Control de regresión |
|---|---|---|
| Al extraer una instalación de OpenGOAL sin exportaciones completas, el importador no encontraba los bancos SBK/VAG que sí existían en la instalación original. Omitía parte de los efectos. | Se enlaza `iso_data` después de extraer, también al reanudar. Se invalida la caché de conversión anterior. El enlace NTFS no cambia permisos; el importador lee los bancos, sin modificarlos. | Extracción nueva y reanudación simuladas, conservación del origen, exportaciones completas y caché. La prueba real del EXE se describe abajo. |
| Coches y zoomers que chocaban con una pared podían empujar un vehículo situado encima cuando fallaba la búsqueda de un escalón. Las sondas reemplazaban el contacto recordado. | Se conserva el tipo de contacto original antes de buscar el escalón; un golpe contra terreno no transfiere su impulso al vehículo de una sonda posterior. | Dos pruebas fallaban con un impulso indebido; ahora registran cero. Otros dos casos conservan el intercambio de impulso en choques directos. No cambian suspensión, velocidades, reloj físico ni ABI. |
| El rescate podía utilizar la última posición segura de otra dimensión, mundo o entidad del jugador. | El registro distingue nivel y jugador y se elimina al desconectar o cerrar servidor. | Cinco de ocho casos fallaban antes; ocho pasan después. Se conservan caída normal, rescate manual y espectadores. |
| Una muerte por eco oscuro en un mundo avanzado podía suprimir sonido y partículas al entrar en otro con un reloj anterior. | Historial de muertes por instancia de nivel, liberado al cerrar el servidor. | Tres de siete casos fallaban antes; siete pasan después. Se conserva la deduplicación de tres segundos y la independencia entre jugadores. |

Además se corrigieron los enlaces de ayuda que se rompían al copiar los cuatro documentos a la raíz del instalador. Dos pruebas de empaquetado fallaban antes y pasan después; los documentos originales del repositorio conservan sus enlaces relativos.

## Pruebas automáticas

- **181 pruebas nativas completas:** 162 del núcleo y 19 del puente JNI, con recursos originales y cero omitidas. Se corrigió una carrera de inicialización entre dos módulos de pruebas, exclusivamente bajo `cfg(test)`.
- **17 pruebas del instalador:** perfiles y cuentas sintéticas, mundo existente, descargas incorrectas, reversión de transacciones fallidas, cambios concurrentes del launcher, caché, audio y mapa único.
- **15 comprobaciones del ciclo de vida Java:** ocho de rescate y siete de eco. Ejecutan código de producción con sustitutos de servidor y salidas; no equivalen a escuchar sonidos ni a comprobar visualmente un rescate entre dos partidas reales.
- **Dos pruebas de enlaces del paquete.**
- **Ocho comprobaciones Java/JNI:** paquetes inválidos, memoria de buffers, colisiones, efectos, presentación de armas, historial de disparos y cámara. La cámara cubre velocidades de 6, 80 y 160 bloques/s, paquetes agrupados o duplicados, parada brusca y teletransporte.
- Compilación Gradle de mod y JAR de distribución correcta. La tarea Gradle `test` indica `NO-SOURCE`; no se cuenta como una suite JUnit.

GitHub ejecuta ahora las pruebas portables en cada cambio. Seis pruebas de partículas necesitan definiciones originales de OpenGOAL: están marcadas explícitamente como dependientes de recursos privados y se ejecutaron localmente. Algunas comprobaciones JNI existentes también dependen de dichos recursos. Una ejecución pública sin ellos no certifica esa parte.

```powershell
python installer/test_install.py
python tools/test_package_docs.py
python tools/test_rescue_lifecycle.py
python tools/test_dark_eco_lifecycle.py
cargo test --locked --manifest-path native/Cargo.toml
# Para la suite completa, configurar JAKMC_ASSETS con el export propio de Jak 3:
cargo test --locked --manifest-path native/Cargo.toml -- --include-ignored
```

Las pruebas Java permiten contrastar el código anterior sin cambiar el checkout: añadir `--revision b4f8bca` a los dos comandos de ciclo de vida.

## Comprobación en Minecraft

Once escenas terminaron con sus comprobaciones previstas satisfechas, en copias temporales del mapa de distribución. Cada prueba guardó resultados y capturas y eliminó su mundo temporal al cerrar. No se modificaron los mundos ni la configuración del perfil habitual.

| Área | Comprobado |
|---|---|
| Integración | Salto desde agua; Flut montado contra muro; explosiones, vegetación, drops y bedrock; ausencia de daño duplicado; morros, volumen y barrido continuo de ocho chasis. |
| Economía | Munición real y bloqueo al agotarse; eco apilado y unidades luminosas completas; cargas guardadas; mochilas; recetas e intercambios; desgaste, reparación y rotura de tabla. |
| Creativo | Las doce armas con inventario vacío; Jak Oscuro y Luminoso sin eco; al volver a supervivencia no aparecen reservas gratuitas. Libro de Keira abierto desde inventario y vuelta al cerrarlo. |
| Reglas y dimensiones | Daño y recogida de vehículos; recetas; reparación; teletransporte y viaje al Nether y vuelta conservando la sesión de Jak; pasos de Jak sin pasos vanilla superpuestos. |
| Vehículos | Saltadunas y Arenatiburón: desplazamiento, cuatro contactos, salto, impulso aéreo, armas y bajada. Zoomers: desplazamiento, altura, carriles, luces, marcha atrás y bajada. La orientación de cámara no saltó en el tramo recto medido. |
| Eco oscuro | Destrucción de objetos, bajas de cerdo/enemigo/Jak y evento de muerte. Probado con Sodium y con el renderizador de Minecraft sin Sodium. |
| Barriles | Barrido de cuatro modelos, desde posiciones altas/bajas y cerca de límites de secciones. Se revisaron capturas con superficies visibles; no hubo excepción de construcción de mallas. |
| Clima | Tormenta de arena, partículas, recuperación de visibilidad y relámpago en bosque. |
| Ciudadanos | Variantes, guardado, descendencia con huevo, navegación, comercio real, sueño, reacción, muerte y retirada. |
| Patada y cajas | Vegetación retirada, cajas rotas y munición de varios colores. Esta escena no constituye una nueva prueba exhaustiva de daño contra todos los tipos de mob. |

Las capturas son controles visuales puntuales. No demuestran por sí solas fluidez de todas las animaciones ni calidad perceptiva del audio. Los nuevos casos de contacto contra un vehículo elevado y cambio de historial entre partidas se reproducen en las pruebas aisladas; las escenas de integración cubren sus sistemas vecinos.

## Instalador y paquete entregado

Se ejecutó el EXE distribuible en una carpeta de Minecraft nueva y aislada, con perfiles sintéticos y exportaciones reales ya disponibles. No fue necesario extraer juegos de nuevo ni disponer de las ISOs originales. Jak 1 reutilizó un export completo de un laboratorio previo; Jak II/3, los exports de OpenGOAL. Por tanto, esta prueba real verifica conversión e instalación desde exports completos; la ruta de extracción nueva se cubre con el fixture indicado arriba.

- Primera instalación: salida 0 en 499,9 s; **14.435 recursos**, **2.830 clips** y **7.348 eventos de sonido**. Ningún export de audio quedó marcado como no disponible.
- Se comprobaron los sonidos `bike_hop`, `pg3nxplo`, `pg3wxplo` y las familias de puertas y ambos coches. Las 517 entradas de clases/DLL coinciden con el JAR de distribución, y su CRC es correcto.
- Mapa único, idéntico al ZIP incluido, y perfil sintético ajeno conservado.
- Segunda ejecución del mismo EXE: salida 0 en 21,5 s. Conservó nombre/icono/argumentos del perfil, configuración, opciones, `level.dat` y un archivo testigo del mundo. No creó otro mapa ni volvió a convertir recursos.
- El perfil creado por ese EXE arrancó en Minecraft: Jak se activó automáticamente en tercera persona, con modelo, pose original y altura de 1,94 bloques; se comprobaron sonidos, ojos de Flut, cubierta de Hellcat y catálogos de ambas ciudades. Se inspeccionó una captura y el cliente cerró normalmente.

SHA-256 del EXE probado: `4fd4772bf91421251b7d1b49f37337b7386b1d0b45250cdb3b0a48dae41b98cc`. El reempaquetado final actualiza únicamente los documentos exteriores y sus enlaces; EXE y payload se verifican sin cambios.

El JAR público no comparte recursos con el árbol privado generado. La revisión del paquete no encontró recursos originales de juegos ni credenciales. El mapa de distribución se mantiene sin datos de jugador; no se incluyeron ISOs. Para instalar el mod siguen haciendo falta los tres juegos instalados en OpenGOAL y conservar sus datos extraídos, no tener las ISOs originales a mano.

## Rendimiento

Equipo: i7-8700K y RTX 2080 Ti, 1920×1080, seis chunks de simulación, VSync desactivado y límite de 260 FPS. Minecraft 26.3; Fabric Loader 0.20.1/API 0.165.0; Sodium 0.9.2, Sodium Extra 0.9.5, Lithium 0.26.3, FerriteCore 9.0.0 y EntityCulling 1.11.3.

| Versión | Chunks | Tramo | FPS medios | P99 (ms) | Pico (ms) | Frames >50 ms |
|---|---:|---|---:|---:|---:|---:|
| Antes | 24 | vista | 49.7 | 31.92 | 190.77 | 1 |
| Antes | 24 | vuelo | 57.6 | 42.96 | 258.89 | 4 |
| Antes | 32 | vista | 51.8 | 34.08 | 116.08 | 3 |
| Antes | 32 | vuelo | 61.0 | 30.63 | 50.58 | 1 |
| Candidato | 24 | vista | 46.2 | 39.08 | 126.70 | 3 |
| Candidato | 24 | vuelo | 60.7 | 42.81 | 235.63 | 4 |
| Candidato | 32 | vista | 53.0 | 37.41 | 55.96 | 3 |
| Candidato | 32 | vuelo | 89.0 | 25.88 | 42.91 | 0 |

Son pasadas cortas de vista y vuelo, de unos ocho segundos por muestra, después de un calentamiento breve. La pasada final se hizo después de terminar la conversión de audio, sin compilación local en paralelo. La variación de carga inicial y cachés impide atribuir las diferencias de FPS al parche: estos cambios no son una optimización del renderizador.

**Siguen existiendo tirones**, especialmente en la primera pasada a 24 chunks (pico de 235,63 ms). No se acredita «24/32 chunks sin tirones». La pasada final de vuelo a 32 chunks no tuvo fotogramas superiores a 50 ms, pero eso no garantiza una sesión larga ni generación de terreno nuevo.

## Límites y evidencia

No se certifican multijugador, servidor dedicado, todas las combinaciones de shaders/mods, sesiones de muchas horas, generación masiva de terreno nuevo ni todas las animaciones y voces. Las pruebas se centraron en la combinación instalada de Sodium, Sodium Extra, Lithium, FerriteCore y EntityCulling, más el contraste indicado sin Sodium.

Las advertencias del registro sobre contadores de rendimiento de Windows y la autenticación del usuario de laboratorio no impidieron las escenas; no se modificó Windows para silenciarlas. Se conserva el mapa personal y sus ajustes.

La evidencia local se guarda en `.local/audit-20261113`: compilación, pruebas de lógica, resultados de las escenas, capturas, rendimiento, instalación real y recibos. No se publican las copias de los recursos originales utilizadas en esas pruebas.
