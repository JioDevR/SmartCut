# Informe de errores y riesgos de SmartCut

Fecha del análisis: 2026-10-17

## Alcance y validación

Se revisaron los flujos de línea de tiempo, cortes, movimiento de clips, reproducción asociada al timeline y exportación en Vue/TypeScript y Rust/Tauri.

Validaciones realizadas:

- `npm run build`: correcto; Vite transformó 74 módulos sin errores.
- `cargo test --manifest-path src-tauri/Cargo.toml`: no ejecutado porque `cargo` no está instalado en el entorno.
- No se editaron archivos de código ni configuración. Este archivo es el único artefacto generado.

## Hallazgos confirmados

### CRÍTICO: los modos de exportación visibles no se aplican

El formulario permite seleccionar `smart_cut`, `reencode` o `individual_clips` en [ExportSettingsForm.vue](src/components/export/ExportSettingsForm.vue#L97-L129). El frontend envía el valor en [useExport.ts](src/composables/useExport.ts#L196-L212), pero `export_video` no inspecciona `options.mode`, `options.video_codec`, `options.hardware_accel`, `options.crf` ni `options.preset`; siempre crea un plan Smart Cut y llama a `execute_smartcut_export` en [engine.rs](src-tauri/src/export/engine.rs#L89-L121).

Impacto:

- Elegir “Renderizado Completo” no recodifica todo el material.
- Elegir “Clips como Archivos Separados” no genera archivos separados.
- El encoder, CRF y preset seleccionados no controlan el comando FFmpeg.
- La interfaz puede mostrar un modo distinto al comportamiento real.

Reproducción: abrir Exportar, seleccionar cualquiera de los dos modos que no sean Smart Cut, iniciar la exportación y comparar el resultado o el progreso con el modo Smart Cut. El mismo camino Rust se ejecuta en ambos casos.

Solución propuesta:

- Despachar explícitamente por `mode` en Rust.
- Implementar un planador separado para `reencode`, usando encoder, CRF y preset recibidos.
- Para `individual_clips`, ejecutar una exportación por segmento con nombres únicos.
- Pasar las opciones efectivas hasta el ensamblador y validar sus valores antes de lanzar FFmpeg.
- Añadir una prueba de integración que compruebe los tres modos, el número de archivos y la duración.

### CRÍTICO: “Extraer solo audio” produce una salida incompatible o falla

El frontend cambia la extensión a `mp3`, `aac` o `flac` y envía `audio_only: true` en [useExport.ts](src/composables/useExport.ts#L196-L212). El backend no usa `audio_only` ni `audio_format` y `execute_smartcut_export` siempre exige piezas de video, concatena video y las mapea como `0:v:0` en [assemble.rs](src-tauri/src/export/assemble.rs#L27-L48) y [assemble.rs](src-tauri/src/export/assemble.rs#L294-L326).

Impacto: la opción de solo audio no puede generar un archivo de audio válido; puede fallar durante el multiplexado o producir un archivo de video con extensión de audio.

Solución propuesta:

- Crear una ruta de exportación de audio independiente que no genere piezas de video ni use `-map 0:v:0`.
- Aplicar el formato elegido (`mp3`, `aac` o `flac`) al codec y al contenedor.
- Validar que exista una pista de audio y devolver un error claro si no existe.
- Verificar con `ffprobe` que la salida contiene solo audio y tiene la duración esperada.

### ALTO: el movimiento de clips no permite reordenarlos

Al comenzar un arrastre, los límites se calculan usando el clip anterior y siguiente ordenados por posición en [useClipDrag.ts](src/composables/useClipDrag.ts#L44-L70). `clipMaxStart` queda limitado a `next.start - dur`, y el nuevo inicio se vuelve a limitar entre `clipMinStart` y `clipMaxStart` en [useClipDrag.ts](src/composables/useClipDrag.ts#L88-L125).

Impacto: un clip no puede cruzar a otro para cambiar el orden de la edición; solo puede desplazarse dentro del espacio disponible. Si el producto pretende una edición no lineal con reordenamiento, la operación falla silenciosamente al llegar al clip vecino.

Nota: conservar `sourceStart/sourceEnd` durante el movimiento es correcto para mover el mismo material dentro del timeline. El problema es que el algoritmo impide moverlo más allá de los vecinos, no que conserve su origen.

Solución propuesta:

- Decidir explícitamente si el producto permite reordenar clips.
- Si se permite, no limitar el arrastre al vecino inmediato; insertar el clip en la nueva posición y ordenar por `start` al terminar.
- Mantener `sourceStart/sourceEnd`, porque representan el material original; solo deben cambiar `start/end`.
- Añadir una prueba de arrastre sobre otro clip que compruebe orden visual, reproducción y exportación.
- Si no se permite reordenar, indicarlo visualmente y mostrar una señal al alcanzar el límite.

### ALTO: la configuración de contenedor no se respeta de forma segura

El selector de contenedor actualiza principalmente la extensión de salida en [useExport.ts](src/composables/useExport.ts#L17-L35) y [useExport.ts](src/composables/useExport.ts#L196-L212). El ensamblador decide algunas opciones únicamente leyendo la extensión en [assemble.rs](src-tauri/src/export/assemble.rs#L26-L38), pero conserva los códecs mediante `-c copy` en el multiplexado final en [assemble.rs](src-tauri/src/export/assemble.rs#L294-L326).

Impacto:

- MP4/MOV/WebM no garantizan compatibilidad con los códecs originales.
- Un video H.264/H.265 con audio o metadatos no compatibles puede fallar al escribir WebM.
- `container_format` enviado por el frontend no se valida ni se usa como política de codificación.

Solución propuesta:

- Validar el contenedor elegido en Rust y definir perfiles de salida por formato.
- Declarar codecs permitidos, audio compatible y flags para cada contenedor.
- Recodificar automáticamente cuando el material no sea compatible o bloquear la combinación con un error explicativo.
- Usar opciones estructuradas para decidir el contenedor; la extensión debe ser solo una consecuencia de esa decisión.

### ALTO: el plan de exportación mostrado puede no coincidir con el que se ejecuta

La vista previa solicita un plan mediante `get_export_plan_preview` en [useExport.ts](src/composables/useExport.ts#L72-L90), mientras que la exportación vuelve a consultar la fuente y los keyframes en [engine.rs](src-tauri/src/export/engine.rs#L45-L68) y [engine.rs](src-tauri/src/export/engine.rs#L89-L112). No hay un identificador ni una comprobación de que ambos análisis usen el mismo estado de archivo, keyframes o segmentos.

Además, el plan de preview se calcula antes de iniciar la exportación y no se recalcula si el estado de clips cambia por otra vía mientras el modal está abierto.

Impacto: las duraciones de “copia directa”, “render” y número de piezas pueden no describir exactamente el proceso real.

Solución propuesta:

- Crear un plan único en Rust y reutilizarlo para preview y ejecución, o devolver un `plan_id` con hash de archivo, segmentos y parámetros.
- Enviar a la preview las mismas opciones que afectan al resultado, incluido modo, contenedor y audio.
- Invalidar la preview cuando cambien segmentos, modo, ruta o ajustes de codificación.
- Antes de exportar, recalcular o validar el plan y evitar ejecutar datos obsoletos.

### ALTO: el planificador fusiona segmentos solapados y cambia el resultado solicitado

`normalize_segments` fusiona segmentos cuando se solapan en [planner.rs](src-tauri/src/export/planner.rs#L51-L88). Esto evita duplicados accidentales, pero también elimina intencionalmente una repetición si el usuario coloca dos clips con rangos de fuente solapados en el timeline.

Impacto: una edición que repita deliberadamente un fragmento no conserva la duración ni el número de apariciones solicitadas; el exportado contiene el rango fusionado una sola vez.

Solución propuesta:

- No fusionar segmentos por defecto: cada clip del timeline debe conservar su aparición y orden.
- Si se necesitan eliminar duplicados accidentales, hacerlo mediante una opción explícita o una regla basada en identidad de clip.
- Limitar la normalización a recortar valores fuera de rango, eliminar segmentos nulos y aplicar una política explícita de solapamiento.
- Añadir una prueba con dos clips que apunten al mismo rango de fuente y comprobar dos apariciones en la salida.

### MEDIO: la eliminación de huecos no actualiza la duración lógica de la fuente/timeline

`closeGap`, `closeGapBeforeClip` y `closeAllGaps` desplazan solo `start/end` y dejan intactos `sourceStart/sourceEnd` en [useClipsManager.ts](src/composables/useClipsManager.ts#L225-L282). El reproductor traduce correctamente el tiempo del timeline al tiempo de fuente mediante esos campos en [useVideoPlayer.ts](src/composables/useVideoPlayer.ts#L204-L224), pero el timeline conserva como duración global la duración original del video.

Impacto observable: después de cerrar todos los huecos, queda un área de timeline posterior al último clip que sigue representando la duración original. Puede parecer un espacio vacío aunque la edición ya esté compactada. Es principalmente un problema de modelo/UX; el exportador sí suma las duraciones de los segmentos.

Solución propuesta:

- Separar explícitamente `sourceDuration` de `timelineDuration`.
- Mantener la duración de fuente para límites de recorte, pero calcular la duración visual como el final del último clip compactado.
- Usar la duración visual para regla, playhead y navegación, y la de fuente para validar `sourceStart/sourceEnd`.
- Recalcularla después de cerrar huecos y cubrir el comportamiento con pruebas.

### MEDIO: el indicador de playhead en hueco puede no reflejar la reproducción real

`isPlayheadInGap` decide si el cabezal está en un hueco comparando `currentTime` con `clip.start/end` en [App.vue](src/App.vue#L204-L213). El reproductor, sin embargo, trabaja con `sourceStart/sourceEnd` y actualiza `currentTime` desde el tiempo de fuente en [useVideoPlayer.ts](src/composables/useVideoPlayer.ts#L329-L363).

Impacto: con clips desplazados o con rangos de fuente no equivalentes a su posición visual, el monitor puede mostrar el estado de “hueco” incorrectamente durante una transición o búsqueda rápida.

Solución propuesta:

- Centralizar una función `findClipAtTimelineTime(time)` y usarla tanto en `isPlayheadInGap` como en la reproducción.
- Comparar el estado del monitor únicamente con `start/end` del timeline; usar los rangos de fuente solo para resolver el seek.
- Actualizar el estado después de cada seek y durante transiciones, con tolerancia de un frame.
- Añadir pruebas para huecos, clips desplazados y el último frame de cada clip.

## Riesgos adicionales

### MEDIO: rangos de fuente no válidos pueden atravesar el final del video

El recorte limita `sourceStart` a cero y aplica un mínimo relativo, pero no limita explícitamente `sourceEnd` a la duración real en [useClipDrag.ts](src/composables/useClipDrag.ts#L154-L193). El exportador finalmente recorta el segmento por `total_duration` en [planner.rs](src-tauri/src/export/planner.rs#L58-L70), lo que puede hacer que la vista del clip y su duración exportada difieran en el borde final.

Solución propuesta:

- Limitar siempre `sourceStart` y `sourceEnd` a la duración real conocida antes de guardar el cambio.
- Mantener la invariancia `0 <= sourceStart < sourceEnd <= sourceDuration`.
- Ajustar o rechazar el gesto de trim cuando llegue al último frame, usando una tolerancia basada en FPS.
- Añadir casos para recortar exactamente al final y más allá del final.

### MEDIO: el nombre y color se basan en el orden actual, no en identidad estable

`renumberClips` renombra y recolorea todos los clips después de mover, cortar o cerrar huecos en [useClipsManager.ts](src/composables/useClipsManager.ts#L145-L155). El `id` permanece estable, pero nombres y colores cambian con cada operación.

Impacto: proyectos guardados, EDL y referencias del usuario pueden cambiar de identidad visual aunque el material sea el mismo.

Solución propuesta:

- Separar el identificador estable del nombre y color de presentación.
- No renombrar ni recolorear clips automáticamente al moverlos o cerrar huecos; reservar el renumerado para una acción explícita.
- Si se necesita mostrar el orden, calcular una etiqueta visual derivada sin sobrescribir el nombre persistido.
- Añadir una prueba de guardar/abrir proyecto que confirme que nombres y colores se conservan.

### BAJO: la verificación de salida no valida la duración esperada ni las pistas esperadas

`verify_output_file` comprueba que FFmpeg pueda decodificar el archivo y obtiene su duración en [verify.rs](src-tauri/src/export/verify.rs#L12-L70), pero no compara esa duración con `plan.total_export_duration`, ni verifica que existan las pistas de audio esperadas. Una salida reproducible pero truncada o sin audio puede declararse correcta.

Solución propuesta:

- Pasar a la verificación la duración esperada, la tolerancia por FPS y el número/tipo de pistas esperado.
- Fallar si la duración se desvía más allá de la tolerancia o si falta audio cuando se solicitó conservarlo.
- Usar `ffprobe` para comprobar streams, codecs, contenedor y duración, no solo que FFmpeg decodifique sin error.
- Conservar el archivo temporal cuando falle la verificación para facilitar el diagnóstico y limpiarlo después de informar el error.

## Resumen de prioridad

1. Corregir el despacho real de `mode`, `audio_only`, formato, encoder, CRF y preset en el backend; actualmente varias opciones de la interfaz son funcionalmente decorativas.
2. Decidir y especificar si mover clips debe permitir reordenamiento. Si sí, cambiar el algoritmo de límites y cubrirlo con pruebas de interacción.
3. Evitar fusionar segmentos solapados cuando representan repeticiones deliberadas; distinguir duplicados accidentales de clips distintos.
4. Validar duración, orden, repetición y pistas de audio en la salida final contra el plan ejecutado.
5. Añadir pruebas Rust y pruebas de composables para cortes, arrastre, cierre de huecos, rangos de fuente y los tres modos de exportación.

## Conclusión

El frontend compila, pero la superficie de exportación presenta fallos funcionales graves porque el contrato de opciones entre Vue y Rust no está implementado de extremo a extremo. El timeline conserva una separación útil entre posición de edición y rango de fuente, aunque el movimiento está restringido y algunos indicadores visuales no usan exactamente el mismo modelo que el reproductor. La exportación no debería considerarse fiable para modos distintos de Smart Cut, solo audio, repeticiones solapadas o contenedores que no sean compatibles con los códecs originales hasta cubrir estos puntos con pruebas de integración.