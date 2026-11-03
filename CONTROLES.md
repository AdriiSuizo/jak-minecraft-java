# Controles actuales del port Java

Abre `PROBAR-JAK.bat`. En la partida: `/jak on`; `/jak equipment` da el equipo si estás en creativo. Usa F5 o el botón Back/View del mando para cambiar de perspectiva. No hace falta instalar otro mod de mando: se usa SDL, incluido en esta versión de Minecraft.

| Acción | PlayStation | Xbox |
|---|---|---|
| Moverse con la lógica de Jak | Stick izquierdo | Stick izquierdo |
| Cámara | Stick derecho | Stick derecho |
| Saltar / doble salto en el aire | X | A |
| Puñetazo | Cuadrado | X |
| Ataque giratorio | Círculo | B |
| Agacharse / rodada al avanzar | L1 | LB |
| Disparar Morph Gun / cargar y soltar Wave Concussor | R1 | RB |
| Usar objeto, puerta o entidad de Minecraft | Triángulo | Y |
| Sacar / guardar turbotabla equipada | R2 | RT |
| Botón original L2 de Jak (modificador según estado) | L2 | LT |
| Poder oscuro / luminoso | L3 / R3 | Pulsar sticks izquierdo / derecho |
| Cambiar perspectiva | Share / Create | Back / View |
| Inventario / creativo | Cruceta arriba | Cruceta arriba |
| Casilla anterior / siguiente | Cruceta izquierda / derecha | Cruceta izquierda / derecha |
| Soltar una unidad | Cruceta abajo | Cruceta abajo |
| Pausa / cerrar pantalla | Options | Start / Menu |
| Cerrar una pantalla | Círculo o Share | B o Back |

Para equipar la turbotabla, selecciónala en la mano y pulsa Triángulo/Y una vez. Se guarda en su propio equipamiento persistente; después R2/RT la saca o guarda. No ocupa el escudo. Empieza el disparo con un Morph Gun seleccionado en la mano. Los poderes necesitan eco.

Se conservan teclado y ratón: B tabla, G puñetazo, V giro, R disparo, Z/X poderes y Espacio salto; se pueden cambiar en las opciones de Minecraft. Las interacciones normales también mantienen clic derecho.

La entrada del mando respeta las zonas muertas originales: el núcleo aplica la del movimiento; la cámara usa zona radial 0,18 y el giro original de 180 grados por segundo a máxima inclinación. Los gatillos se leen analógicamente; los botones digitales originales usan presión 0,75 y liberación 0,65. El mando se detecta al conectarlo, libera sus botones al desenchufarlo y suspende movimiento/ataques en menús o al perder foco.

Abrir/cerrar inventario, cambiar casillas e interactuar ya está integrado. Mover el cursor y elegir todas las casillas de los menús con el mando todavía no está implementado. Los controles de coches/zoomers R2 acelerar, L2 frenar/marcha atrás, X altura y Cuadrado freno de mano corresponden a la siguiente integración de vehículos; no son las acciones de Jak a pie de esta tabla.
