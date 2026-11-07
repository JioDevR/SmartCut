# Informe VF: segunda auditoría de SmartCut

Fecha: 2026-10-17

## Alcance y método

Se volvió a seguir el flujo completo:

1. Modelo de clips: posición en timeline frente a rango de fuente.
2. Cortes, trim, movimiento, huecos, reproducción, undo/redo y apertura de proyectos.
3. Conversión de clips a segmentos de exportación.
4. Plan Smart Cut, keyframes, piezas de video, audio, multiplexado, cancelación y verificación.

Se comparó el comportamiento real del código con lo que anuncia la interfaz y con las reglas de FFmpeg. No se modificó código del sistema.

Validación disponible:

- `npm run build`: correcto.
- `cargo test --manifest-path src-tauri/Cargo.toml`: no ejecutable en este entorno porque `cargo` no está instalado.
- Por esa limitación, los fallos de FFmpeg y Rust se confirman por inspección del comando generado y deben cerrarse con pruebas de integración sobre archivos reales.

## Hallazgos confirmados

### VF-01 | Crítico: el backend ignora el modo de exportación y varios ajustes

La UI permite `smart_cut`, `reencode` e `individual_clips` en [ExportSettingsForm.vue](src/components/export/ExportSettingsForm.vue#L97-L129), y el frontend envía esas opciones en [useExport.ts](src/composables/useExport.ts#L196-L212). Sin embargo, `export_video` siempre normaliza segmentos, crea un plan Smart Cut y llama a `execute_smartcut_export` en [engine.rs](src-tauri/src/export/engine.rs#L89-L121).

También quedan sin efecto real `hardware_accel`, `video_codec`, `crf` y `preset`. El ensamblador obtiene el codec desde la fuente mediante [encoder.rs](src-tauri/src/export/encoder.rs#L3-L18), no desde la selección del usuario.

Impacto: “Renderizado Completo” y “Clips como Archivos Separados” no hacen lo que indican; el encoder, calidad y preset seleccionados no controlan la exportación.

Solución:

- Despachar por modo en Rust antes de crear el plan.
- Implementar una ruta de recodificación completa y otra que genere un archivo por segmento.
- Pasar opciones validadas hasta el constructor de comandos FFmpeg.
- Añadir pruebas de integración que distingan los tres modos por salida, codec, número de archivos y duración.

### VF-02 | Crítico: la exportación de solo audio no está implementada

El frontend cambia la extensión y envía `audio_only`/`audio_format` en [useExport.ts](src/composables/useExport.ts#L196-L212), pero el backend no los usa. `execute_smartcut_export` exige piezas de video en [assemble.rs](src-tauri/src/export/assemble.rs#L27-L48), mapea `0:v:0` y vuelve a mapear video en el multiplexado final en [assemble.rs](src-tauri/src/export/assemble.rs#L294-L326).

Impacto: la salida puede fallar o contener video con una extensión de audio. La opción de audio no es funcional.

Solución: crear una ruta exclusiva de audio, sin piezas de video ni `-map 0:v:0`, aplicar explícitamente el codec elegido y verificar con `ffprobe` que la salida contiene solo audio.

### VF-03 | Crítico: se acortan las piezas Copy respecto a la duración del plan

El plan declara `piece.duration_sec`, pero para una pieza `Copy` el ensamblador resta `trim_margin` y además adelanta el seek con `seek_offset` en [assemble.rs](src-tauri/src/export/assemble.rs#L72-L112). Aun cuando FFmpeg produzca exactamente `safe_dur`, la duración real es menor que la duración declarada por el plan.

El progreso y `plan.total_export_duration` siguen usando la duración original. `verify_output_file` solo comprueba decodificación y obtiene la duración, sin compararla con el plan en [verify.rs](src-tauri/src/export/verify.rs#L12-L70).

Impacto: pérdida de frames o de tiempo en cada tramo copiado, duración final menor y posible desfase audio/video acumulado.

Solución: definir con precisión qué frames se descartan, compensar esos descartes en las piezas contiguas o generar el tramo completo mediante un método que preserve la duración; después comparar duración esperada y real con tolerancia de uno o dos frames.

### VF-04 | Crítico: índice global de stream usado como índice de video ordinal

`probe_source_info` guarda el índice global de `ffprobe` en `VideoStreamInfo.stream_index` en [probe.rs](src-tauri/src/export/probe.rs#L96-L111). Después ese valor se usa como `v:N` en `-select_streams` desde [packets.rs](src-tauri/src/export/packets.rs#L18-L30), y como `0:v:N` al generar piezas en [assemble.rs](src-tauri/src/export/assemble.rs#L68-L77).

En FFmpeg, `0:v:N` y `-select_streams v:N` usan el ordinal dentro de los streams de video, mientras que `stream_index` puede ser el índice global del archivo. En un archivo con audio, subtítulos, imágenes o más de un video, el video seleccionado puede no ser `v:0` aunque su índice global sea `1` o mayor.

Impacto: no se encuentran keyframes o FFmpeg falla con “stream not found”; afecta archivos válidos cuya pista de video no es el primer stream global.

Solución: mantener por separado `global_stream_index` y `video_ordinal`; usar ordinal para `v:N` y el índice global solo para un mapeo absoluto compatible. Añadir un fixture con audio antes del video, subtítulos y dos streams.

### VF-05 | Alto: segmentos fuera de orden rompen la consulta de keyframes

El exportador conserva el orden de los clips para producir la secuencia, pero calcula el rango de inspección como `normalized.first().start` y `normalized.last().end` en [engine.rs](src-tauri/src/export/engine.rs#L50-L56) y [engine.rs](src-tauri/src/export/engine.rs#L104-L110). Si la secuencia contiene rangos de fuente como `[10, 12]` seguido de `[2, 4]`, `min_start` es `10` y `max_end` es `4`.

Impacto: la consulta de keyframes recibe un intervalo invertido, devuelve datos incompletos o falla; el problema aparece especialmente con reordenamiento, proyectos importados o repeticiones no monotónicas.

Solución: calcular `min(start)` y `max(end)` solo para el rango de análisis, pero conservar el orden original para ensamblar segmentos. Añadir una prueba de exportación con dos clips en orden de fuente inverso.

### VF-06 | Alto: se fusionan repeticiones legítimas de un mismo rango

`normalize_segments` fusiona dos segmentos cuando se solapan en [planner.rs](src-tauri/src/export/planner.rs#L51-L88). El backend no conoce si el solapamiento es accidental o si representa dos clips distintos que deben aparecer dos veces.

Impacto: una repetición intencional desaparece y la duración exportada deja de coincidir con el timeline.

Solución: conservar cada segmento como una instancia de edición y no fusionar por defecto. Si se requiere deduplicación, debe ser una opción explícita y no la conducta normal del exportador.

### VF-07 | Alto: el audio no es realmente sample-accurate

El comentario del ensamblador promete precisión de muestra, pero cada corte usa `-ss` antes de `-i` en [assemble.rs](src-tauri/src/export/assemble.rs#L157-L178). Ese modo permite seek rápido aproximado; después se limita con `-t` y se reencoda, pero no se aplica un trim exacto posterior al punto solicitado.

Impacto: el inicio de audio puede desplazarse y producir desfase con el video, sobre todo en formatos con timestamps o keyframes separados.

Solución: usar seek de entrada solo como optimización y después recortar timestamps/muestras con una etapa exacta, o usar `-ss` después de `-i` cuando el coste sea aceptable. Comparar el primer timestamp y duración de audio y video en pruebas.

### VF-08 | Alto: la configuración de contenedor se traduce solo a extensión

El selector cambia el sufijo mediante [useExport.ts](src/composables/useExport.ts#L17-L35), mientras que el ensamblador decide parte del comportamiento leyendo la extensión en [assemble.rs](src-tauri/src/export/assemble.rs#L26-L38) y multiplexa con `-c copy` en [assemble.rs](src-tauri/src/export/assemble.rs#L294-L326).

Impacto: seleccionar WebM, MP4 o MOV no garantiza que los codecs y pistas sean compatibles con ese contenedor. El fallo depende del archivo de entrada, por lo que es un riesgo confirmado del diseño, no necesariamente un fallo para todos los videos.

Solución: validar combinaciones contenedor/codec antes de exportar y definir perfiles de salida. Recodificar cuando sea necesario o bloquear la combinación con un mensaje claro.

### VF-09 | Alto: el plan preview no es el mismo artefacto que se ejecuta

La preview y la exportación hacen dos análisis separados en [useExport.ts](src/composables/useExport.ts#L72-L90), [engine.rs](src-tauri/src/export/engine.rs#L45-L68) y [engine.rs](src-tauri/src/export/engine.rs#L89-L121). No hay `plan_id`, hash de entrada ni validación de que los keyframes y opciones sigan siendo los mismos.

Impacto: la información mostrada puede quedar obsoleta o no reflejar el plan final. El riesgo es real aunque no se manifieste si el archivo y los clips permanecen estáticos.

Solución: crear el plan una sola vez en Rust, devolver un identificador y ejecutar ese plan; invalidarlo si cambian archivo, segmentos, modo, contenedor o ajustes.

### VF-10 | Medio: la verificación acepta salidas truncadas o con pistas incorrectas

`verify_output_file` verifica tamaño y decodificación, pero no compara duración, streams, codecs, orden de pistas ni presencia de audio con las expectativas del plan en [verify.rs](src-tauri/src/export/verify.rs#L12-L70).

Solución: pasar a la verificación duración esperada, tolerancia, número de pistas y tipo de salida; usar `ffprobe` para validar esos campos y conservar el temporal cuando falle el diagnóstico.

### VF-11 | Medio: falta validación de entrada sin video

El planificador puede crear piezas con FPS por defecto aunque `source.video` sea `None`; después el ensamblador intenta mapear `0:v:0` en [assemble.rs](src-tauri/src/export/assemble.rs#L27-L48) y [assemble.rs](src-tauri/src/export/assemble.rs#L68-L77).

Solución: validar al inicio que exista una pista de video para exportación de video y derivar audio-only a su ruta propia. El error debe indicar qué tipo de medio se requiere.

### VF-12 | Medio: los clips solapados en tiempo de fuente hacen ambiguo el playback

La reproducción busca el primer clip cuyo rango de fuente contiene `video.currentTime` en [useVideoPlayer.ts](src/composables/useVideoPlayer.ts#L329-L363) y [useVideoPlayer.ts](src/composables/useVideoPlayer.ts#L530-L559). Si dos clips distintos reutilizan o solapan la misma zona de fuente, no existe una correspondencia única entre tiempo de fuente y tiempo de timeline.

Impacto: el playhead puede saltar al clip equivocado, detenerse en el límite equivocado o reproducir una instancia distinta de la esperada.

Solución: mantener una relación de reproducción por instancia de clip, no buscar solo por tiempo de fuente; durante un clip activo, aceptar únicamente su rango y saltar explícitamente al siguiente clip al terminar.

### VF-13 | Medio: abrir un proyecto no restaura completamente el estado del editor

Al abrir proyecto, [App.vue](src/App.vue#L390-L421) reemplaza `clips`, `splitPoints`, `inPoint` y `outPoint`, pero no restaura `selectedClipId`, ni valida que los clips pertenezcan a la duración y medio cargados. `splitAtTime` además busca en `clips.value` sin ordenar en [useClipsManager.ts](src/composables/useClipsManager.ts#L164-L211).

Impacto: puede quedar una selección inexistente, el inspector puede mostrar estado anterior y un corte posterior puede actuar sobre un clip incorrecto si el archivo contiene clips fuera de orden.

Solución: validar y normalizar el proyecto al abrirlo, ordenar clips, seleccionar un clip válido o limpiar la selección y comprobar rangos de fuente contra la duración detectada.

## Hallazgos anteriores reclasificados

### Movimiento de clips

Se confirma que el algoritmo no permite cruzar clips vecinos en [useClipDrag.ts](src/composables/useClipDrag.ts#L44-L125). Eso es un bug solo si el producto promete reordenamiento; por sí mismo puede ser una decisión de edición que evita solapamientos. Debe documentarse o implementarse explícitamente, pero no debe presentarse como fallo absoluto sin esa definición.

Conservar `sourceStart/sourceEnd` durante un desplazamiento sí es correcto: mueve la instancia en el timeline sin cambiar el material de origen.

### Playhead en huecos y duración visual

El uso de `start/end` para saber si el playhead está en un hueco es conceptualmente correcto, porque esos valores representan el timeline. La discrepancia con `sourceStart/sourceEnd` no prueba por sí sola un bug. Queda como riesgo de transición y debe comprobarse con pruebas de clips desplazados, huecos y reproducción real.

La duración global que conserva la duración de la fuente también puede ser una decisión de UX. Es recomendable separar `sourceDuration` y `timelineDuration`, pero no es un fallo de exportación confirmado.

### Renombrado y colores

`renumberClips` cambia nombres y colores al ordenar. Esto puede ser indeseable para identidad visual, pero no rompe el material ni el exportado por sí mismo. Debe clasificarse como decisión de producto o mejora de persistencia, no como bug crítico.

## Orden recomendado de corrección

1. Corregir índices globales/ordinales de streams y comprobar existencia de video.
2. Corregir la duración real de piezas Copy y verificar duración/pistas de salida.
3. Separar rutas y contratos para Smart Cut, recodificación completa, clips individuales y audio-only.
4. Corregir rango global de keyframes para segmentos no monotónicos y no fusionar repeticiones legítimas.
5. Corregir seek exacto de audio y validar sincronía.
6. Crear un plan único para preview y ejecución.
7. Validar apertura de proyectos y definir formalmente si el arrastre permite reordenar.

## Pruebas mínimas necesarias

- Archivo con audio antes del video, subtítulos y más de un stream de video.
- Segmentos en orden de timeline distinto al orden de fuente.
- Dos clips que repitan exactamente el mismo rango de fuente.
- Video con B-frames, GOP largo y audio AAC/Opus/FLAC.
- Exportación Smart Cut comparando duración planificada y duración real frame a frame.
- Los tres modos de exportación, cada uno con una aserción distinta.
- Solo audio con y sin pista de audio.
- Cancelación durante cada etapa: pieza, concat de video, corte de audio y multiplexado.
- Proyecto guardado con clips fuera de orden, selección y rangos cercanos al final del medio.

## Conclusión

La segunda auditoría confirma que los problemas más graves están en el contrato de exportación y en la precisión del ensamblado: opciones de UI no implementadas, duración declarada distinta de duración producida, selección incorrecta de streams y análisis de keyframes inválido para secuencias no monotónicas. Los problemas de movimiento y playhead requieren separar claramente bugs de implementación de decisiones de producto. La exportación no debe considerarse fiable para todos los medios hasta corregir VF-01 a VF-07 y cubrir las pruebas de integración indicadas.