# Informe Técnico V2: Diagnóstico de Desincronización de Audio y Empalme Anómalo en Exportación Smart Cut

**Fecha:** 14 de Octubre de 2026  
**Proyecto:** SmartCut (Tauri + Vue 3 + FFmpeg)  
**Objetivo:** Análisis lógico y trazabilidad de código sobre la desincronización de audio al mover clips en la línea de tiempo y la falla de empalme en los cortes de inicio (HEAD vs BODY).

---

## 1. Resumen Ejecutivo del Problema

El usuario reporta dos síntomas críticos que ocurren conjuntamente tras realizar una edición habitual en la línea de tiempo:
1. **Acción en la Timeline:** Al borrar un clip previo y adelantar el clip siguiente hacia el segundo 0 (cerrar el espacio vacío / ripple manual en el canal único), al exportar se produce una **desincronización notable de audio**.
2. **Falla en el Empalme de Inicio:** El segmento inicial re-codificado (`HEAD` de precisión) se visualiza correctamente en cuanto a video pero **se reproduce sin audio (mudo)**, y justo al terminar esa sección inicial, **se empalma con una versión del clip que parece no haberse cortado** (salta al inicio del archivo original o a un punto desfasado), ocurriendo este fallo específicamente en los **inicios** de los clips cortados.

Tras una inspección exhaustiva del código en el frontend (`useClipDrag.ts`, `useClipsManager.ts`, `useExport.ts`, `useVideoPlayer.ts`) y en el motor de exportación en Rust (`assemble.rs`, `planner.rs`, `packets.rs`, `engine.rs`), se han identificado las **tres causas raíz lógicas** que provocan este comportamiento.

---

## 2. Análisis del Lado del Timeline (Frontend)

### 2.1. Modelo de Datos de Clips: Timeline vs Archivo Fuente
En `src/types/index.ts` y `useClipsManager.ts`, un clip tiene dos pares de coordenadas temporales:
- `start` y `end`: Posición espacial y duración dentro de la **línea de tiempo visual**.
- `sourceStart` y `sourceEnd`: Marcas de tiempo de entrada y salida dentro del **archivo original de video**.

### 2.2. ¿Qué ocurre al borrar un clip y adelantar el siguiente?
1. **División:** El usuario corta en el segundo `10.0`.
   - Clip 1: `[start: 0, end: 10, sourceStart: 0, sourceEnd: 10]`
   - Clip 2: `[start: 10, end: 60, sourceStart: 10, sourceEnd: 60]`
2. **Borrado:** El usuario elimina el Clip 1.
   - Queda Clip 2 en la timeline con un hueco de `0.0s` a `10.0s`.
3. **Movimiento / Arrastre (Adelantar el clip a 0):**
   - En `useClipDrag.ts` (líneas 118-119):
     ```typescript
     clip.start = newStart;      // Ahora pasa a ser 0.0s
     clip.end = newStart + dur;  // Ahora pasa a ser 50.0s
     ```
   - **Observación:** `clip.sourceStart` (10.0s) y `clip.sourceEnd` (60.0s) se mantienen intactos, lo cual es conceptualmente correcto porque el contenido del video sigue siendo el segmento de 10s a 60s del archivo original.
4. **Traspaso a la Exportación:**
   - En `useExport.ts` (función `getActiveSegments`):
     ```typescript
     return cachedClips.value.map((c) => ({
       start: c.sourceStart !== undefined ? c.sourceStart : c.start,
       end: c.sourceEnd !== undefined ? c.sourceEnd : c.end,
       name: c.name,
     }));
     ```
   - Al backend de Rust le llega:
     `segments = [{ start: 10.0, end: 60.0 }]`.

> **Conclusión del Frontend:** El frontend traslada correctamente las coordenadas de origen (`sourceStart` y `sourceEnd`). El problema se detona en cómo el backend interpreta, corta y ensambla estas coordenadas.

---

## 3. Diagnóstico Profundo en el Backend (Rust / FFmpeg)

### Causa Raíz 1: Discrepancia Crítica de Timestamps (PTS) entre `HEAD` y `BODY`
*(Explica por qué "lo empalma con una versión del clip que parece no haberse cortado")*

Para exportar el segmento de `10.0s` a `60.0s`, el planificador (`planner.rs`) calcula:
- Primer keyframe usable dentro del rango: `kf = 14.0s`.
- Último keyframe usable: `kl = 50.0s`.
- Genera 3 piezas de video:
  - **Pieza 0 (HEAD - Re-encode):** de `10.0s` a `14.0s` (duración `4.0s`).
  - **Pieza 1 (BODY - Stream Copy):** de `14.0s` a `50.0s` (duración `36.0s`).
  - **Pieza 2 (TAIL - Re-encode):** de `50.0s` a `60.0s` (duración `10.0s`).

En `src-tauri/src/export/assemble.rs`:

#### 1. Extracción de HEAD (Pieza 0):
```rust
ffmpeg -y -noautorotate -ss 10.000000 -i input.mp4 -t 4.000000 \
  -map 0:v:0 <encoder_args> -an -f mpegts video_piece_0000.ts
```
Al decodificar y re-codificar con `libx264`/`libx265`, el encoder **reinicia los timestamps a 0**:
- `video_piece_0000.ts` tiene fotogramas con **PTS de 0.0s a 4.0s**.

#### 2. Extracción de BODY (Pieza 1):
```rust
ffmpeg -y -ss 14.001000 -i input.mp4 -t 36.000000 \
  -map 0:v:0 -c:v copy -an -f mpegts video_piece_0001.ts
```
**Aquí radica la falla:**
- Al usar `-c:v copy` hacia un contenedor MPEG-TS sin `-avoid_negative_ts make_zero` ni `-copyts -start_at_zero`:
- Los paquetes de video copiados **conservan el PTS original del contenedor fuente MP4**.
- Por lo tanto, `video_piece_0001.ts` **no empieza en 0.0s**, sino con **PTS = 14.0s** (1.260.000 ticks en 90kHz).

#### 3. Concatenación en Concat Demuxer:
```rust
ffmpeg -y -fflags +genpts -f concat -safe 0 -i video_list.txt \
  -c copy -avoid_negative_ts make_zero video_concat.ts
```
El demuxer `concat` lee:
- Archivo 0 (`video_piece_0000.ts`): duración 4.0s (PTS 0.0s → 4.0s).
- Archivo 1 (`video_piece_0001.ts`): sus paquetes internos dicen PTS = 14.0s.
- Concat demuxer le suma el offset acumulado (4.0s) al PTS base (14.0s) o intenta reconciliar con `+genpts`.
- **Efecto Visual:** El reproductor muestra los primeros 4 segundos (HEAD), pero en el empalme (segundo 4.0), el timeline de video se descalabra:
  - O salta temporalmente produciendo un salto brusco.
  - O si el seek previo de `-ss 14.001` cayó en el keyframe 0 por problemas de seek con `-c copy`, **la pieza copiada arranca desde el segundo 0 del video original**, repitiendo el inicio y pareciendo que el clip **nunca se cortó**.

---

### Causa Raíz 2: Desincronización de Audio y el "Clip Renderizado sin Audio"
*(Explica por qué el clip inicial se ve mudo y el audio queda fuera de tiempo)*

#### 1. Las piezas de video son exclusivamente video (`-an`):
En `assemble.rs`, todas las piezas intermedias (`.ts`) se extraen con `-an` (sin audio). El audio no existe en el flujo de video intermedio.

#### 2. La extracción de Audio por Segmento:
En `assemble.rs` (líneas 187-203):
```rust
ffmpeg -y -ss 10.000000 -i input.mp4 -t 50.000000 \
  -map 0:1 -c:a copy -vn audio_s0_cut_0000.mka
```
**El problema técnico de FFmpeg con `-ss` y `-c:a copy`:**
- Cuando se coloca `-ss` **antes de `-i`** con copia directa de audio (`-c:a copy`) en un archivo MP4 con video y audio entrelazados:
- El demuxer de MP4 (`mov.c` en FFmpeg) realiza un salto a nivel de contenedor buscando el **Keyframe de video más cercano anterior**.
- Si el keyframe de video anterior estaba en `0.0s`, el demuxer empieza a entregar paquetes de audio desde el segundo `0.0s`.
- A diferencia del video decodificado (donde FFmpeg descarta los frames entre 0s y 10s), con `-c:a copy` **FFmpeg no puede descartar muestras de audio intermedias sin decodificar**.
- En consecuencia, el archivo `audio_s0_cut_0000.mka` contiene audio que empieza varios segundos antes del punto exacto `10.0s`.

#### 3. Omisión de Normalización cuando hay un solo clip:
En `assemble.rs` (líneas 219-222):
```rust
if audio_seg_files.len() == 1 {
    let single_file = temp_dir.file_path(&audio_seg_files[0]);
    let _ = std::fs::copy(single_file, &audio_concat_path);
}
```
Si el usuario exporta 1 segmento (el clip adelantado):
- El archivo se copia directamente por sistema de archivos, **omitiendo la concatenación de audio que contenía `-avoid_negative_ts make_zero`**.
- El archivo `.mka` llega al mux final con un timestamp de inicio desalineado respecto al video.

#### 4. Mux Final Desfasado:
```rust
ffmpeg -y -fflags +genpts -i video_concat.ts -i audio_s0_concat.mka \
  -map 0:v:0 -map 1:a:0 -c copy -avoid_negative_ts make_zero output.mp4
```
- `video_concat.ts` arranca en PTS `0.0s` (mostrando el corte desde 10.0s).
- `audio_s0_concat.mka` tiene paquetes de audio que o bien arrancan en el keyframe anterior (desincronizados por varios segundos), o bien arrancan con un PTS que no coincide con el video.
- **Resultado perceptible:** Durante los primeros segundos del video (el HEAD), el audio no se escucha o reproduce un tramo desfasado; al llegar al empalme de BODY, el audio entra desfasado respecto a la imagen.

---

### Causa Raíz 3: Orden de Segmentos y Clips Múltiples en `normalize_segments`
En `planner.rs` (línea 76):
```rust
valid.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(std::cmp::Ordering::Equal));
```
Si en la línea de tiempo el usuario tiene varios clips y los mueve o cambia de orden (por ejemplo, clip de la escena final colocado al principio de la pista V1):
- `normalize_segments` **vuelve a ordenar los clips por su `sourceStart`**, destruyendo el orden que el usuario organizó en la línea de tiempo.
- Aunque en un único canal contiguo sin reordenar no altere el orden, la fusión de clips solapados (`merged`) asume contigüidad cronológica del archivo original, no de la línea de tiempo.

---

## 4. Tabla Comparativa de Causa y Efecto

| Síntoma Observado por el Usuario | Ubicación en el Código | Causa Técnica Real |
|---|---|---|
| **"El clip renderizado funciona pero sin audio"** | `assemble.rs:79, 104, 187` | Las piezas `.ts` se generan con `-an`. El audio extraído en `.mka` tiene un timestamp inicial desalineado con el video de 0s; el reproductor silencia o desincroniza el tramo de HEAD. |
| **"Lo empalma con una versión del clip que no parece cortada"** | `assemble.rs:88-111` y `planner.rs:139` | HEAD es re-encode (PTS base 0); BODY es copy en MPEG-TS (PTS base absoluto del original). Al unirlos en Concat Demuxer hay una discontinuidad masiva de PTS o un fallo de seeking que hace que BODY empiece desde el inicio del video. |
| **"El audio se desincroniza al mover el clip hacia adelante"** | `assemble.rs:189, 219` | `-ss` antes de `-i` con `-c:a copy` salta al keyframe de video previo (no al tiempo exacto de audio). Además, con 1 clip se omite `-avoid_negative_ts make_zero` al copiar el archivo `.mka` directo con `std::fs::copy`. |
| **Solo pasa en los inicios** | `planner.rs:127-147` | Los finales (TAIL) no tienen el problema de que una pieza `Copy` arranque en un PTS arbitrario empalmado después de un `Encode` base 0 sin alinear. Ocurre exactamente en la unión `HEAD (Encode) -> BODY (Copy)`. |

---

## 5. Plan de Acción Recomendado (Solución de Ingeniería)

### Paso 1: Normalización de Timestamps en Piezas de Video
Para que `HEAD` (re-encode) y `BODY` (copia) se concatenen de forma 100% transparente sin saltos ni descalabro temporal:
1. En `PieceKind::Encode`: Asegurar que el filtro `setpts=PTS-STARTPTS` mantenga los fotogramas estrictamente lineales.
2. En `PieceKind::Copy`: Añadir `-avoid_negative_ts make_zero` al comando de extracción de cada pieza `.ts`, o generar cada pieza con timestamps normalizados a 0 para que el concat demuxer sume exactamente la duración de cada tramo sin depender de los PTS internos del MP4 original.

### Paso 2: Extracción Precisa de Audio (Audio Frame-Accurate)
El audio AAC/Opus/MP3 no puede cortarse con precisión de milisegundos usando `-c:a copy` con `-ss` previo a `-i` porque salta a keyframes de video:
1. **Solución Óptima:** Extraer el audio del segmento decodificándolo y codificándolo con el mismo códec y bitrate original (`-c:a aac -b:a <bitrate_original>`) o usando `-af "atrim=start=...:end=...,asetpts=PTS-STARTPTS"`. Al ser audio, el re-encode tarda fracciones de segundo (decenas de veces más rápido que tiempo real) y garantiza **precisión a nivel de muestra (sample-accurate)** y perfecta sincronía labial.
2. Si se mantiene `-c:a copy`: Colocar `-ss` **después de `-i`** para que FFmpeg decodifique el contenedor hasta el segundo exacto antes de demuxear paquetes de audio, e incluir siempre `-avoid_negative_ts make_zero`.

### Paso 3: Garantizar Timestamp Cero en Audio MKA
Eliminar el bypass de `std::fs::copy` cuando hay 1 solo clip en `assemble.rs:219`. Siempre procesar el audio resultante para garantizar que el primer paquete empiece en `PTS = 0.000000`, alineado de forma idéntica al primer frame del video.

### Paso 4: Respetar el Orden de la Línea de Tiempo
En `planner.rs`, los segmentos deben procesarse en el **orden en que aparecen en la línea de tiempo** del usuario (`clip.start`), sin reordenarlos automáticamente por `sourceStart`.
