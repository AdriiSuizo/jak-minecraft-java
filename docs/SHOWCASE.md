# Mundo de showcase

Un mundo normal (no ultraplano) con Spargus y Villarefugio uno al lado del otro. Entre las dos ciudades hay un camino con una jincana de estaciones para enseñar el mod poco a poco.

## Cómo abrirlo
1. Descarga `JakShowcase-world.zip` de la última versión en [Releases](https://github.com/AdriiSuizo/jak-minecraft-java/releases).
2. Extrae el ZIP en `%APPDATA%\.minecraft\instances\jakmod\saves`.
3. Abre la instalación **26.3.jakmod** y entra en **Jak Showcase**.

Empiezas en la salida, mirando hacia la primera estación. El mundo está en modo supervivencia, con los comandos activados, siempre de día, sin lluvia y sin monstruos naturales. Si mueres no pierdes lo que llevas.

## Las estaciones
Cada una tiene su título flotante y un cartel. Sigue el camino en orden.

| Estación | Qué enseña |
|---|---|
| Salida | El punto de partida: el camino lleva hasta el espacio entre las dos ciudades |
| 1 Patada y movimientos | Hierba y flores que se rompen con la patada, un anillo de bloques para la patada giratoria, un muro de salientes de tres alturas y una piscina |
| 2 Objetos | Barriles de cada tierra, cajas de munición, lámparas colgantes, la puerta iris y la esclusa |
| 3 Eco | Las seis fuentes de eco, el lanzador azul, un foso de eco oscuro como el del silo de Jak 1 (mata al instante y deshace lo que caiga) y un cofre de eco |
| 4 Armas | Dos cofres con las 12 armas y la munición, un corral de lurkers y un muro para destrozar |
| 5 Jak Oscuro y Luminoso | Cofre con gemas y eco, y una arena con Metal Heads |
| 6 Bichos | Todas las criaturas en vitrinas de cristal; rompe el cristal para soltarlas |
| 7 Vehículos | HellCat, Hell Dog, coches del desierto, zoomers, Flut Flut, yakow, la turbotabla y un riel para deslizarse |
| 8 Ciudades | El cruce hacia Spargus y hacia Villarefugio |

En Villarefugio las puertas de la torre y del edificio de la Guardia son esclusas, y dentro de los edificios hay cajas de munición.

## Comandos útiles
- Modo creativo: `/gamemode creative`.
- Quitar los títulos flotantes para grabar sin ellos: `/kill @e[type=text_display]`.
- Hacer de noche: `/time set night`.
- Volver a la salida: `/tp @s` seguido de las coordenadas que muestra `/spawnpoint`, o morir una vez.
- Activar monstruos naturales: `/gamerule spawn_monsters true`.

## Cómo se genera (para desarrollo)
La escena `showcase` lo construye sobre un mundo nuevo:

```bash
python tools/run_scene.py --scene showcase
```

Después `tools/pack_showcase.py` empaqueta el mundo que deja en `.local/evidence/showcase-<fecha>/world`. El código está en `src/client/java/dev/jakmc/client/ShowcaseScene.java`.
