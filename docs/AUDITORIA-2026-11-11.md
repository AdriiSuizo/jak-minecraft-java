![Jak Worlds](media/jak-worlds-header.gif)

[Inicio](../README.md) · [Descargar](https://github.com/AdriiSuizo/jak-minecraft-java/releases/tag/v0.1.0-dev.18) · [Guías](../docs/README.md)

---

# Auditoría de Jak en Minecraft · 11 de noviembre de 2026

## Entorno

Windows, Intel Core i7-8700K, NVIDIA GeForce RTX 2080 Ti y Java 25 con un máximo de 6 GB de heap. Las dos instalaciones ejecutan Minecraft 26.3 con Fabric Loader 0.20.1, Fabric API 0.165.0+26.3, Sodium 0.9.2, Sodium Extra 0.9.5, Lithium 0.26.3, FerriteCore 9.0.0 y EntityCulling 1.11.3. Movie añade Replay Mod 26.3-2.6.28.

Las pruebas de juego se hicieron con copias independientes de **Jak Showcase** y de la configuración de cada perfil. Se midió a 1920 × 1080 con seis chunks de simulación, VSync desactivado y un límite de 260 FPS para evitar que la configuración o la inactividad ocultaran diferencias. Se tomaron tramos de ocho segundos de vista fija y de vuelo por Villarefugio después de una espera de unos diez segundos en cada distancia. El servidor integrado se midió en paralelo.

## Rendimiento

| Perfil | Chunks | Tramo | FPS medios | P99 de fotograma | Máximo | Fotogramas >50 ms |
|---|---:|---|---:|---:|---:|---:|
| Normal anterior | 24 | Vista | 67,9 | 24,64 ms | 43,29 ms | 0 |
| Normal anterior | 24 | Vuelo | 77,1 | 33,26 ms | 537,29 ms | 3 |
| Normal anterior | 32 | Vista | 63,8 | 24,41 ms | 67,37 ms | 3 |
| Normal anterior | 32 | Vuelo | 81,9 | 23,88 ms | 73,33 ms | 1 |
| Normal actualizado | 24 | Vista | 95,2 | 21,62 ms | 41,97 ms | 0 |
| Normal actualizado | 24 | Vuelo | 99,2 | 22,08 ms | 121,23 ms | 3 |
| Normal actualizado | 32 | Vista | 89,4 | 19,24 ms | 41,59 ms | 0 |
| Normal actualizado | 32 | Vuelo | 101,7 | 21,03 ms | 50,19 ms | 1 |
| Movie actualizado | 24 | Vista | 90,9 | 21,55 ms | 40,14 ms | 0 |
| Movie actualizado | 24 | Vuelo | 95,5 | 23,15 ms | 119,23 ms | 4 |
| Movie actualizado | 32 | Vista | 91,4 | 18,50 ms | 54,08 ms | 1 |
| Movie actualizado | 32 | Vuelo | 102,8 | 20,22 ms | 44,57 ms | 0 |

El P99 es el tiempo que el 99 % de los fotogramas igualó o mejoró. En la serie de 24 a 32 chunks, los ticks más largos del servidor integrado actualizado fueron de 32,58 ms en normal y 36,53 ms en Movie, dentro del presupuesto de 50 ms por tick. La ruta original registró carga de modelos de enemigos y generación del atlas de gemas en el hilo de renderizado durante los tirones. En las compilaciones actualizadas quedan picos cercanos a 120 ms al inicio de la primera pasada de vuelo.

Se invirtió el orden en una segunda ejecución del perfil normal para medir la primera pasada a 32 chunks. La vista dio 64,7 FPS, P99 de 25,28 ms y un pico de 53,41 ms. El vuelo dio 84,3 FPS, P99 de 37,31 ms y un pico de 210,18 ms. En la vista hubo un tick de servidor de 148,39 ms. La pasada posterior a 24 chunks dio 98,2 FPS en la vista y 109,1 FPS en vuelo, con máximos de 43,74 y 43,24 ms. La fluidez al empezar la ruta a 32 chunks sigue siendo el principal problema de rendimiento observado.

## Cambios

- Las correcciones de Replay se compilan ahora desde el repositorio. La variante Movie graba una pista de poses en el archivo de repetición y la utiliza durante la reproducción y la exportación. El reloj del vídeo gobierna la interpolación de Jak y los vehículos y el avance de los efectos. El JAR normal carga sin Replay Mod.
- Con Sodium, la cámara usa la colisión de terreno de Minecraft y el eco oscuro usa el sprite de reflejos con tinte violeta. En una instalación que dibuja el terreno por la ruta de Minecraft se mantiene el shader de líquido y el recorte de la cámara en interiores. El acabado del líquido y el encuadre en interiores pueden diferir según el renderizador.
- El renderizador GLB reutiliza colores de materiales sin cambios y reduce la preparación de las caras opacas. La carga de modelos remapea índices con matrices de enteros y aplica el color del material a los vértices que utiliza cada malla. El atlas de gemas reutiliza el búfer de interpolación y sus 245 760 píxeles coinciden con el original.
- El lanzador de pruebas conserva los mundos anteriores y archiva el que creó. La prueba de reglas espera a que el servidor procese el viaje de ida y vuelta antes de cerrar. La de clima fuerza la carga del terreno y coloca al jugador por encima de la superficie antes de medir las partículas.

## Verificación

- 177 pruebas nativas y cinco del instalador superadas. Entre las nativas hay pruebas de colisiones de zoomers y del radio de Super Nova. Las 48 referencias de recetas del libro de Keira resuelven a recursos del JAR. El libro se abrió con su botón en el inventario de supervivencia y volvió al inventario al cerrarlo. El auditor del JAR del instalador encontró cero coincidencias con recursos privados.
- En juego se comprobó el gasto de munición en supervivencia y su bloqueo con el cargador vacío, el consumo de eco y los intercambios del mod, el desgaste y la reparación del JET-Board, la conducción de dos zoomers y las bajas en el foso de eco oscuro. Se revisó la captura del líquido con Sodium y se vieron partículas sobre la superficie.
- En creativo, las doce armas dispararon con el inventario vacío. También se activaron Jak Oscuro y el vuelo de Jak Luminoso sin eco en el inventario. Al volver a supervivencia, las reservas reales de munición y eco seguían vacías.
- La prueba de reglas confirmó que Jak conserva su sesión al teletransportarse y al viajar al Nether y volver. Registró once pasos de Jak y cero pasos de Minecraft superpuestos. La de clima confirmó partículas de arena, recuperación de la visibilidad al escampar y un relámpago nocturno en bosque.
- La prueba de integración comprobó el salto desde el agua, el montaje y la colisión del Flut contra un muro, el daño a bloques y las colisiones de los ocho chasis terrestres y flotantes. Una prueba adicional verificó que la patada deja intactos la salud de un cerdo y los bloques sólidos de su entorno.
- La prueba de Movie abrió una repetición existente y exportó el salto de eco azul a 1280 × 720 y 60 FPS. FFprobe leyó los 171 fotogramas y la instrumentación detectó partículas del lanzador. Una grabación de partida de 58 segundos generó un archivo íntegro con 5 196 muestras de movimiento, incluidas 4 040 de vehículos.
- La prueba del foso de eco oscuro pasó también sin Sodium y Sodium Extra. Se revisó una captura del reflejo del shader de Minecraft y otra del sprite usado por Sodium.

## Reproducir la compilación

Desde la raíz del proyecto con Java 25 y las dependencias del proyecto disponibles.

```powershell
.\gradlew.bat jar distributionJar
.\gradlew.bat movieJar "-PjakmcReplayJar=$env:APPDATA/.minecraft/instances/jakmodmovie/mods/replaymod-26.3-2.6.28.jar"
py tools/audit_distribution.py
```

El JAR `distributionJar` es el que se valida para el instalador y este importa los recursos desde la copia del propietario. Las instalaciones locales ya contenían audio importado y nueve alias de recetas heredados que no están en la compilación de desarrollo actual. La actualización local parte de los recursos presentes en cada instalación y verifica su integridad al incorporar el código y los recursos versionados.

Se instalaron los dos JAR verificados en `jakmod` y `jakmodmovie`. Cada actualización conserva 17 254 entradas de recursos locales y deja una copia del JAR anterior en `.local/backups/audit-20261111`. Ambos perfiles conservan la configuración de 32 chunks de renderizado y seis de simulación.

## Alcance

La matriz corresponde al hardware, versiones y rutas indicadas. Quedan fuera de estas mediciones una sesión larga, un mundo nuevo en plena generación, un servidor dedicado, el multijugador y las combinaciones con otros shaders. La exportación de Replay probada es de vídeo. La mezcla de audio del tráiler requiere su propia comprobación.
