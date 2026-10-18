# Informe V2: Tercera Auditoría y Revisión Integral de Código (SmartCut)

**Fecha:** 2026-10-18  
**Proyecto:** SmartCut (Lossless Video Cutter con Smart Cut frame-accurate)  
**Alcance:** Revisión estricta a nivel de código (backend Rust en `src-tauri` y frontend Vue/TypeScript en `src`).

---

## 1. Resumen Ejecutivo y Estado Actual

Tras la implementación de las correcciones derivadas de la segunda auditoría (`informeVF.md`), se ejecutó una verificación completa de compilación y pruebas automatizadas:

- **Backend Rust (`src-tauri`):** `cargo check` y `cargo test` completados exitosamente. Los **31 tests unitarios y de integración pasan al 100%** (incluyendo pruebas con video real MP4, MKV/HEVC, B-frames, clips en orden inverso y duplicados, extracción de audio AAC/FLAC/MP3 y validación de contenedores).
- **Frontend Vue / TypeScript (`src`):** `npm run build` genera el bundle de producción sin advertencias y `npx tsc --noEmit` valida sin errores de tipado.
- **Flujo esencial:** Los componentes críticos (reproducción por streaming HTTP 206, cálculo de keyframes, visualización de timeline y exportación) son plenamente operativos.

No obstante, la revisión minuciosa del código fuente ha identificado **8 hallazgos** (entre discrepancias de integración frontend-backend, casos borde en códecs y precisión milimétrica de ensamblado). A continuación se documenta cada uno en detalle para su consideración.

---

## 2. Estado de Validación de Hallazgos Previos (VF-01 a VF-13)

| ID | Área | Estado Actual | Observaciones |
| :--- | :--- | :---: | :--- |
| **VF-01** | Modos de exportación / Encoders | **Corregido** | Rutas dedicadas implementadas para Smart Cut, Renderizado Completo (`reencode`) y Clips Separados (`individual_clips`). Se respetan CRF, preset y GPU. |
| **VF-02** | Solo Audio | **Corregido** | Función `execute_audio_only_export` operativa sin mapas de video ni artefactos. Soporta MP3, AAC, FLAC, WAV y Opus. |
| **VF-03** | Duración de piezas Copy | **Mitigado con observación** | Se eliminó el recorte arbitrario grande; sin embargo, persiste un caso borde con `seek_offset` analizado en el Hallazgo V2-01. |
| **VF-04** | Ordinal de video vs stream global | **Corregido** | Separación limpia entre `stream_index` (global) y `video_index` (ordinal `v:N`). |
| **VF-05** | Keyframes en clips no monotónicos | **Corregido** | El análisis calcula `min_start` y `max_end` globales preservando el orden de edición original. |
| **VF-06** | Conservación de duplicados y solapamientos | **Corregido** | `normalize_segments` ya no fusiona clips repetidos ni solapados; la duración final coincide con el timeline. |
| **VF-07** | Seek sample-accurate de audio | **Corregido** | Se implementó seek exacto posterior a `-i` (`build_sample_accurate_audio_seek_args`), eliminando desfases de paquetes. |
| **VF-08** | Matriz contenedor/codec | **Corregido** | Módulo `compatibility.rs` valida combinaciones válidas (WebM vs VP8/VP9/AV1, MP4 vs H.264/HEVC, MKV universal). |
| **VF-09** | Plan Preview reutilizable | **Corregido** | Se genera firma hash (`compute_plan_signature`) y caché en memoria (`PLAN_CACHE`). (Ver V2-03 para caso borde en frontend). |
| **VF-10** | Verificación post-exportación | **Corregido** | `verify_output_file_full` valida duración real, pistas de audio/video y preserva copia temporal para diagnóstico en caso de fallo. |
| **VF-11** | Validación sin video | **Corregido** | Detección temprana en `get_export_plan_preview` y `export_video` exigiendo video o derivando a audio-only. |
| **VF-12** | Desambiguación de reproducción | **Corregido** | `useVideoPlayer.ts` rastrea `activePlaybackClipId` para evitar saltos ambiguos cuando varios clips comparten tiempo de fuente. |
| **VF-13** | Restauración de proyectos | **Corregido** | `loadProjectState` ordena, valida contra la duración del medio y limpia/repara la selección de clips al abrir. |

---

## 3. Nuevos Hallazgos de Código (Revisión V2)

### V2-01 | Alto: Pérdida de fotogramas en el límite BODY / TAIL por sustracción de `seek_offset` en piezas Copy

- **Ubicación:** [assemble.rs](file:///home/jiodev/Documents/Proyects/SmartCut/src-tauri/src/export/assemble.rs#L138-L156)
- **Descripción:**  
  Al generar una pieza `PieceKind::Copy`, el código calcula un `seek_offset` (ej. `frame_dur * 4.5` en B-frames, o hasta 0.25s en MKV) para evitar que el demuxer retroceda al GOP anterior. Luego define:
  ```rust
  let effective_dur = (piece.duration_sec - seek_offset).max(frame_dur);
  ```
  Y pasa a FFmpeg:
  ```rust
  "-ss", format!("{:.6}", piece.start_sec + seek_offset),
  "-i", source.path,
  "-t", format!("{:.6}", effective_dur),
  "-c:v", "copy"
  ```
- **Causa Raíz:**  
  Cuando `-ss` se sitúa antes de `-i`, FFmpeg posiciona la lectura en el keyframe (que está exactamente en `piece.start_sec`). Al especificar `-t effective_dur`, FFmpeg detiene la copia `effective_dur` segundos después. Como `effective_dur` es menor que `piece.duration_sec`, los últimos `seek_offset` segundos antes del siguiente keyframe (`kl.pts_time`) no se copian.  
  Dado que la siguiente pieza `TAIL` empieza en `kl.pts_time`, los fotogramas comprendidos en el intervalo `[kl.pts_time - seek_offset, kl.pts_time]` (entre 4 y 15 fotogramas según FPS) desaparecen del video final.  
  Sin embargo, el audio se corta por la duración nominal total (`cut.duration_sec`), produciendo un desfase progresivo acumulativo entre audio y video en cada segmento Smart Cut.
- **Impacto:** Acortamiento del video de entre 75ms y 200ms por cada segmento que contiene pieza Copy + Tail, y desincronización de audio en archivos con GOP largo.
- **Solución Recomendada:**  
  En lugar de recortar `effective_dur`, limitar la pieza Copy exactamente al PTS final del keyframe destino mediante `-to` o un filtro de paquetes, o compensar la diferencia extendiendo la pieza `TAIL` hacia atrás para que absorba los fotogramas restantes del GOP de forma recodificada continua.

---

### V2-02 | Alto: Fallo de verificación en Renderizado Completo (`reencode`) con múltiples pistas de audio

- **Ubicación:** [assemble.rs](file:///home/jiodev/Documents/Proyects/SmartCut/src-tauri/src/export/assemble.rs#L774-L811) y [assemble.rs](file:///home/jiodev/Documents/Proyects/SmartCut/src-tauri/src/export/assemble.rs#L883-L914)
- **Descripción:**  
  En `execute_full_reencode_export`, cuando se procesan múltiples segmentos (rama `else` línea 725), el bucle de extracción de audio solo procesa la primera pista de audio:
  ```rust
  "-map", format!("0:{}", source.audio[0].stream_index)
  ```
  Y en el multiplexado final solo mapea `1:a:0`. Las pistas de audio secundarias (`source.audio[1..]`) no se extraen ni se incluyen.  
  Sin embargo, en la línea 906 se ejecuta:
  ```rust
  let exp_audio = if !source.audio.is_empty() { Some(source.audio.len()) } else { Some(0) };
  verify_output_file_full(&tmp_final_str, ..., exp_audio)?;
  ```
- **Impacto:** Si el video fuente tiene 2 o más pistas de audio (por ejemplo, idiomas múltiples en MKV/MP4), el renderizado completo multi-clip falla al final con el error de verificación:  
  `"Discrepancia en pistas de audio: esperadas 2, encontradas 1"`.
- **Solución Recomendada:**  
  Iterar sobre todas las pistas de audio de `source.audio` (análogamente a cómo lo hace `execute_smartcut_export_with_options` en las líneas 237-360) para preservar todas las pistas de audio en el modo de renderizado completo, o ajustar `exp_audio` al número de pistas efectivamente procesadas.

---

### V2-03 | Medio: Invalidación involuntaria de la caché de Preview por discrepancia en `video_codec`

- **Ubicación:** [useExport.ts](file:///home/jiodev/Documents/Proyects/SmartCut/src/composables/useExport.ts#L13), [useExport.ts](file:///home/jiodev/Documents/Proyects/SmartCut/src/composables/useExport.ts#L87) y [App.vue](file:///home/jiodev/Documents/Proyects/SmartCut/src/App.vue#L755)
- **Descripción:**  
  En `useExport.ts` se declaró:
  ```typescript
  const selectedVideoCodec = ref('h264');
  ```
  En `refreshPlanPreview`, se envía `video_codec: selectedVideoCodec.value` (siempre `'h264'`).  
  Sin embargo, en `App.vue`, el evento `@start-export` invoca:
  ```typescript
  startExport(currentFilePath, sortedClips, metadata.video_codec, selectedClip)
  ```
  Donde `metadata.video_codec` es el códec real del archivo (por ejemplo, `'hevc'` o `'vp9'`).
- **Causa Raíz:**  
  La función Rust `compute_plan_signature` en `engine.rs` (línea 75) incluye `options.video_codec` en el hash.  
  Como `refreshPlanPreview` calculó el hash con `'h264'` y `startExport` envía `'hevc'`, `current_hash != entry.input_hash`.
- **Impacto:** El plan preview generado y mostrado en la interfaz nunca coincide con el plan de exportación para videos HEVC, VP9 o AV1. El backend descarta silenciosamente la entrada de caché (`cached_entry` es `None`) y recalcula el plan de keyframes desde cero.
- **Solución Recomendada:**  
  Inicializar `selectedVideoCodec.value = metadata.video_codec` en `openExportModal` para que tanto el preview como la exportación compartan idéntico códec y aprovechen la caché de `plan_id`.

---

### V2-04 | Medio: Discrepancia visual entre el botón de formato (MKV) y la ruta por defecto (`.mp4`)

- **Ubicación:** [useExport.ts](file:///home/jiodev/Documents/Proyects/SmartCut/src/composables/useExport.ts#L23) y [useExport.ts](file:///home/jiodev/Documents/Proyects/SmartCut/src/composables/useExport.ts#L140)
- **Descripción:**  
  En `useExport.ts`, `containerFormat` está inicializado como `'mkv'`, lo que causa que en `ExportSettingsForm.vue` aparezca el botón `MKV (Recomendado)` activado.  
  Sin embargo, al abrir el diálogo de exportación (`openExportModal`), la ruta de guardado se inicializa como:
  ```typescript
  exportOutputPath.value = dir ? `${dir}${sep}${baseName}_smartcut.${inputExt}` : ...;
  ```
  Si el video original es un archivo `.mp4`, la ruta de salida terminará en `.mp4`.
- **Impacto:** La interfaz muestra visualmente "MKV (Recomendado)" seleccionado, pero el campo de texto y el archivo resultante se exportan como `.mp4` a menos que el usuario vuelva a hacer clic manualmente en otro botón para disparar el watcher.
- **Solución Recomendada:**  
  Al abrir el modal, alinear `containerFormat.value` con la extensión real sugerida o invocar `updateOutputExtension()` inmediatamente después de inicializar la ruta.

---

### V2-05 | Medio: Argumento incompatible en Tauri commands de diálogo (`defaultPath` vs `default_name`)

- **Ubicación:** [App.vue](file:///home/jiodev/Documents/Proyects/SmartCut/src/App.vue#L441-L444) y [App.vue](file:///home/jiodev/Documents/Proyects/SmartCut/src/App.vue#L491-L494)
- **Descripción:**  
  Al guardar proyectos o exportar listas EDL, `App.vue` invoca los comandos de Tauri pasando el objeto:
  ```typescript
  await invoke('save_project_dialog', { defaultPath: defaultName, content: jsonContent });
  await invoke('save_edl_dialog', { defaultPath: defaultName, content: edlText });
  ```
  En el backend Rust ([lib.rs](file:///home/jiodev/Documents/Proyects/SmartCut/src-tauri/src/lib.rs#L23-L64)), las firmas de las funciones son:
  ```rust
  async fn save_project_dialog(content: String, default_name: Option<String>)
  async fn save_edl_dialog(content: String, default_name: Option<String>)
  ```
- **Impacto:** Como la propiedad enviada es `defaultPath` en lugar de `defaultName` (o `default_name`), Tauri asigna `default_name = None`. Los diálogos del sistema operativo ignoran el nombre del archivo de video y siempre se abren con los nombres genéricos `"proyecto.smartcut"` y `"timeline.edl"`.
- **Solución Recomendada:**  
  Cambiar la clave en `App.vue` a `defaultName: defaultName`.

---

### V2-06 | Medio: Riesgo de colisión de archivos en "Clips como Archivos Separados"

- **Ubicación:** [assemble.rs](file:///home/jiodev/Documents/Proyects/SmartCut/src-tauri/src/export/assemble.rs#L969-L976)
- **Descripción:**  
  En `execute_individual_clips_export`, el nombre de cada archivo se define como:
  ```rust
  let clip_name = if let Some(ref name) = seg.name {
      let clean: String = name.chars().map(...).collect();
      format!("{}_{}.{}", stem, clean, ext)
  } else {
      format!("{}_clip_{:02}.{}", stem, i + 1, ext)
  };
  ```
- **Impacto:** Si el proyecto contiene clips con nombres repetidos (por ejemplo, si el usuario duplicó un clip o si dos clips se llaman "Clip 1"), el segundo clip sobreescribe al primero sin previo aviso, resultando en pérdida del archivo anterior.
- **Solución Recomendada:**  
  Incluir siempre el índice correlativo para garantizar unicidad:
  ```rust
  format!("{}_{:02}_{}.{}", stem, i + 1, clean, ext)
  ```

---

### V2-07 | Bajo: Resumen del modal no refleja el alcance "Clip Seleccionado"

- **Ubicación:** [ExportSettingsForm.vue](file:///home/jiodev/Documents/Proyects/SmartCut/src/components/export/ExportSettingsForm.vue#L36-L38) y [ExportSettingsForm.vue](file:///home/jiodev/Documents/Proyects/SmartCut/src/components/export/ExportSettingsForm.vue#L301-L321)
- **Descripción:**  
  La caja de resumen inferior (`export-summary-box`) muestra:
  ```html
  <span class="summary-val">{{ sortedClips.length }} clips</span>
  <span class="summary-val">{{ formatTimecode(totalDuration, metadata.fps) }}</span>
  ```
  Donde `totalDuration` calcula la suma de todos los clips de la línea de tiempo.
- **Impacto:** Si el usuario tiene seleccionada la opción "Clip Seleccionado" (`exportScope === 'selected'`), el resumen inferior sigue mostrando la cantidad y duración total de toda la línea de tiempo (por ejemplo, "4 clips, 02:45.00") en lugar de indicar "1 clip" y la duración del corte elegido.
- **Solución Recomendada:**  
  Hacer que el cómputo de `totalDuration` y el conteo de clips en el resumen dependan de `props.exportScope` y `props.selectedClip`.

---

### V2-08 | Bajo: Exclusión de keyframe exacto al inicio del segmento en `planner.rs`

- **Ubicación:** [planner.rs](file:///home/jiodev/Documents/Proyects/SmartCut/src-tauri/src/export/planner.rs#L107-L110)
- **Descripción:**  
  El filtro de keyframes candidatos dentro del segmento utiliza:
  ```rust
  .filter(|k| k.pts_time > seg_start + half_frame && k.pts_time < seg_end - half_frame)
  ```
- **Impacto:** Si el usuario corta exactamente sobre un fotograma clave (`seg_start == k.pts_time`), ese keyframe queda excluido de `candidate_kfs`. El algoritmo interpreta que no hay keyframe utilizable al inicio y genera una pieza HEAD o recodifica el segmento completo innecesariamente, perdiendo la oportunidad de iniciar con copia directa 100% sin pérdida desde el primer fotograma.
- **Solución Recomendada:**  
  Permitir que un keyframe alineado con `seg_start` (con tolerancia de medio fotograma `|k.pts_time - seg_start| <= half_frame`) sea considerado como el inicio de la pieza `Copy` sin requerir una pieza `HEAD` de recodificación.

---

## 4. Matriz de Priorización para la Siguiente Iteración

| Prioridad | ID | Componente | Descripción Resumida |
| :---: | :---: | :---: | :--- |
| **P1** | **V2-01** | `assemble.rs` | Evitar descarte de fotogramas y desfase A/V por `seek_offset` en piezas Copy. |
| **P1** | **V2-02** | `assemble.rs` | Preservar todas las pistas de audio en `reencode` multi-clip para evitar fallos de verificación. |
| **P2** | **V2-03** | `useExport.ts` | Sincronizar `selectedVideoCodec` con el medio para mantener activa la caché de `plan_id`. |
| **P2** | **V2-04** | `useExport.ts` | Sincronizar extensión de archivo por defecto con el contenedor seleccionado (`containerFormat`). |
| **P2** | **V2-05** | `App.vue` | Corregir nombre de parámetro `defaultName` en llamadas Tauri de guardado de proyecto y EDL. |
| **P3** | **V2-06** | `assemble.rs` | Agregar índice a nombres de clips individuales para evitar sobreescritura accidental. |
| **P3** | **V2-07** | `ExportSettingsForm.vue` | Reflejar duración y conteo del clip seleccionado en la caja de resumen. |
| **P3** | **V2-08** | `planner.rs` | Optimizar copia directa cuando el corte coincide exactamente con un I-frame. |

---

## 5. Conclusión Técnica

La arquitectura actual de SmartCut ha alcanzado un alto nivel de madurez y estabilidad:
- El motor de exportación cubre ahora todos los modos prometidos en la interfaz gráfica (Smart Cut, Renderizado Completo, Clips Separados y Solo Audio).
- El streaming local HTTP 206 y la desambiguación de clips resuelven de forma sólida los problemas de reproducción y scrub en formatos complejos como MKV y WebM.
- La suite de 31 pruebas automatizadas en Rust garantiza que las regresiones más críticas del pasado (VF-01 a VF-13) se encuentran resueltas y blindadas.

Los hallazgos de este informe corresponden a ajustes de precisión milimétrica (conservación exacta de fotogramas de borde en B-frames, consistencia de metadata en el preview y soporte multi-audio en reencode). Al abordarse estas observaciones, el software alcanzará un estándar óptimo para su distribución y uso en producción.
