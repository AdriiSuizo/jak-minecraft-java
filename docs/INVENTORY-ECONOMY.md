# Inventario, eco, munición y turbotabla

El servidor usa los objetos transportados como única reserva. Una unidad de eco oscuro aporta cuatro puntos y una de eco luminoso dos; el círculo original de Jak III muestra hasta cien puntos por lado. El consumo se descuenta automáticamente durante las acciones. El vuelo luminoso también necesita y consume eco. Creativo conserva munición infinita, como el resto de armas de Minecraft; los poderes necesitan eco transportado.

Se cuentan las ranuras normales y la mano secundaria, los componentes nativos de contenedores (incluidas shulkers y bolsas) y los inventarios extraíbles que las mochilas expongan mediante Fabric ItemStorage/ContainerItemContext. Esto no certifica todas las implementaciones privadas de mochilas de otros mods. No se consulta el cofre de Ender ni cofres colocados en el mundo.

Las fracciones restantes viven en un objeto real, individual y persistente, con CustomData. No hay crédito oculto cuando se tira o pierde el objeto. Una fracción extraída de un contenedor vuelve al inventario exterior, o se deja caer si está lleno; así no aumenta silenciosamente el peso de una bolsa llena. Su descripción indica el porcentaje que queda y que fabricar/comerciar requiere una unidad completa. Estas fracciones no son herramientas reparables y no sirven como unidades completas para fabricar o comprar munición. Las reservas numéricas de versiones anteriores se convierten una sola vez en objetos reales al entrar.

## Recetas sin forma

Todas usan una unidad completa de eco y una del segundo ingrediente. Se descubren en el recetario al conseguir el eco correspondiente.

| Munición | Eco | Segundo ingrediente | Resultado |
|---|---|---|---:|
| Roja | Luminoso | Redstone | 16 |
| Amarilla | Luminoso | Lingote de oro | 16 |
| Azul | Luminoso | Lapislázuli | 64 |
| Morada | Oscuro | Fragmento de amatista | 8 |

Las cuatro usan los modelos originales gun-ammo-* tanto al caer como en el inventario. El encuadre del icono se ajusta a sus dimensiones, conservando las cuatro formas. Los materiales opacos de armas y cargadores ignoran el alfa de reflexión del exportador. Los colores de la textura specular original se conservan mediante un tinte de material barato; esto no implementa el BRDF/reflejo completo de PS2. Los efectos del eco mantienen sus grupos originales; la composición del icono convierte la emisión aditiva en transparencia, evitando el cuadrado negro del render target de objetos.

Al recoger munición suena el efecto original ammo-pickup.

Los enemigos de Jak conservan sus piedras calavera y eco; los de Jak II/III también pueden soltar munición. Los guardias sueltan munición, eco luminoso y, raramente, su bláster. Los ciudadanos mantienen su entidad y comercio nativos de Minecraft: venden las cuatro municiones por eco, eco por esmeraldas y compran piedras calavera. Las ofertas se guardan, no se duplican al reabrir y reponen la mercancía de recursos cada día, incluso sin estación de trabajo.

La turbotabla tiene 1800 puntos de durabilidad: gasta uno cada segundo de desplazamiento efectivo. Irrompibilidad procesa el desgaste con las reglas de Minecraft; Reparación consume los orbes de experiencia para reparar la tabla de su ranura propia. Su icono y la barra nativa aparecen junto al hotbar al equiparla. Al romperse termina el estado de montar; una nueva tabla vuelve a equiparse mediante su uso normal. La tabla equipada recibe Reparación antes que el equipo convencional; el XP sobrante sigue el proceso nativo.

## Encontrar las ciudades

`/jak locate haven` y `/jak locate spargus` muestran coordenadas de generación normal. `/jak where` muestra la posición actual. También funcionan los comandos nativos `/locate structure jakmc:haven` y `/locate structure jakmc:spargus` con los permisos de Minecraft. Las construcciones aparecen en chunks nuevos o que ya contengan su inicio guardado; no se sobrescriben construcciones personales ni se regeneran mapas existentes. Spargus sigue restringida al desierto.

## Verificación

La escena economy verifica dentro del servidor real inventarios anidados, fracciones y persistencia, las cuatro recetas y el rechazo de eco gastado, los intercambios, el desgaste, los encantamientos, la rotura, el consumo de munición, el bloqueo del arma vacía y la reparación con un orbe real. La escena natural_cities localiza los inicios naturales y comprueba fachadas, puertas y cofres en bosque y desierto. Las imágenes se revisan además de los resultados numéricos.
