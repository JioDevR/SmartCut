# Informe de Análisis Técnico: Motor de Exportación y Renderizado Smart Cut

**Fecha de análisis:** 14 de Octubre de 2026  
**Proyecto:** SmartCut Tauri  
**Archivo analizado:** `src-tauri/src/export/engine.rs` (y componentes vinculados en `src-tauri` y `src/composables/useExport.ts`)  
**Problemas reportados:**
1. Falta de precisión en los cortes al exportar o renderizar en modo **Smart Cut**.
2. Corrupción de datos periódica, artefactos visuales (glitches, macrobloques) y fallos de reproducción al avanzar rápido (*fast scrubbing / seek*) en diversos reproductores (VLC, MPV, Windows Media Player, QuickTime).

---

## 1. Resumen Ejecutivo y Diagnóstico General

El concepto de **Smart Cut** (o *Smart Rendering*) consiste en permitir cortes con precisión de fotograma (*frame-accurate*) minimizando la recodificación:
- **Cuerpo central del clip:** Se copia directamente sin pérdida de calidad (*stream copy* con `-c copy`).
- **Bordes inicial y final:** Dado que un corte rara vez coincide exactamente con un fotograma clave (*Keyframe / IDR*), únicamente se deben recodificar los fotogramas entre el punto de corte y el fotograma clave adyacente.

Tras una inspección profunda del código en `src-tauri/src/export/engine.rs`, se han identificado **múltiples fallos estructurales graves** tanto en la segmentación matemática de los cortes como en la multiplexación y formato de los flujos de video/audio generados:

| Categoría | Causa Principal | Impacto Directo |
| :--- | :--- | :--- |
| **Falta de Precisión** | Ventana de tolerancia arbitraria de 80 ms (`0.08s`) | Pérdida o adición de entre 2 y 5 fotogramas en los cortes. |
| **Falta de Precisión** | Uso de `-ss` previo a `-i` con `-c copy` | Desplazamiento inexacto del inicio del cuerpo copiado. |
| **Falta de Precisión** | Solapamiento de fotogramas en los límites de los slices | Duplicación de fotogramas clave (*stutter* y saltos temporales). |
| **Corrupción de Video** | Mezcla de flujos con diferentes SPS/PPS en MP4 sin bitstream filter | Artefactos verdes, macrobloques rotos y decodificación errónea en GPU. |
| **Corrupción en Scrubbing** | Detección de I-frames comunes en vez de IDR (Open GOP) | Referencias rotas en fotogramas B huérfanos al avanzar rápido. |
| **Corrupción de Video** | Discontinuidades de DTS/PTS y timebases dispares | Pérdida de monotonicidad y destrucción de índices de búsqueda (`stts`/`ctts`). |
| **Corrupción de Audio** | Mezcla de audio recodificado (AAC 256k) con audio original copiado | Saltos de sincronía, chasquidos (*pops*) y fallo del decodificador de audio. |

---

## 2. Análisis Detallado: ¿Por qué no tiene precisión en modo Smart Cut?

### 2.1. La ventana de tolerancia de 80 ms (`0.08s`) en `plan_smart_slices`
En `engine.rs` (líneas 554-582 y 595-612), la lógica de planificación de slices contiene las siguientes condiciones:
```rust
let k_first = keyframes.iter().copied().find(|&k| k > seg_start + 0.08);
let k_last = keyframes.iter().copied().rfind(|&k| k < seg_end - 0.08);

if kf - seg_start > 0.08 {
    slices.push(SmartSlice {
        start: seg_start,
        duration: kf - seg_start,
        reencode: true,
        stage_desc: "Borde inicial (precisión frame-accurate)".to_string(),
    });
}
```
**¿Qué ocurre aquí?**
- A 30 fps, cada fotograma dura **33.3 ms**. A 60 fps, cada fotograma dura **16.6 ms**.
- Un umbral de **80 ms** equivale a **2.4 fotogramas a 30 fps** y a **casi 5 fotogramas a 60 fps**.
- Si el usuario define un corte que queda a menos de 80 ms de un fotograma clave, el algoritmo **omite por completo el slice de recodificación** y ancla el corte directamente al fotograma clave.
- **Resultado:** El video exportado comienza o termina en un fotograma distinto al elegido por el usuario en la línea de tiempo.

### 2.2. Inexactitud del comando `-ss` previo al input con `-c copy`
En la generación de comandos para el fragmento copiado (Slice de cuerpo principal):
```rust
let mut s_cmd = vec![
    "-y".to_string(),
    "-ss".to_string(),
    format!("{:.3}", slice.start),
    "-i".to_string(),
    input_path.to_string(),
    "-t".to_string(),
    format!("{:.3}", slice.duration),
];
// Con -c copy ...
```
- Cuando `-ss` se coloca antes de `-i` junto con `-c copy`, FFmpeg realiza un *fast seek* a nivel de demuxer hacia el paquete que considera más cercano.
- Al no decodificar los paquetes, no existe garantía de que el primer paquete copiado sea exactamente el fotograma clave calculado por `ffprobe`.
- Si el valor decimal `{:.3}` redondea milisegundos hacia arriba o hacia abajo, FFmpeg puede posicionarse en el fotograma clave anterior (arrastrando segundos indeseados del GOP previo) o saltarse el fotograma clave inicial.

### 2.3. Duplicación de fotogramas en la frontera entre Recodificación y Copia
Analicemos la frontera temporal entre el borde inicial y el cuerpo principal:
- **Slice 1 (Borde inicial recodificado):** Inicia en `seg_start` y dura `kf - seg_start`. Pretende llegar exactamente hasta el instante anterior a `kf`.
- **Slice 2 (Cuerpo copiado):** Inicia en `kf` y dura `kl - kf`.

Dado que FFmpeg evalúa la condición de parada del argumento `-t` comparando el timestamp del fotograma con la duración acumulada, las pequeñas discrepancias de punto flotante provocan que:
1. El fotograma ubicado en `kf` sea decodificado y codificado dentro del **Slice 1**.
2. El fotograma clave en `kf` sea extraído en el flujo copiado del **Slice 2**.
3. Al concatenar ambos slices, **el mismo fotograma aparece dos veces consecutivas**, produciendo un micro-congelamiento o salto (*stutter* perceptible) justo en el punto de transición.

---

## 3. Análisis Detallado: ¿Por qué el archivo generado tiene corrupción de datos y falla al pasar rápido?

Este es el problema más grave y evidente para los usuarios finales. Los reproductores multimedia no muestran una simple pérdida de precisión, sino **macrobloques de colores verdes o grises, roturas de textura e imposibilidad de hacer búsqueda rápida (*scrubbing*)**.

### 3.1. Incompatibilidad de Parámetros de Secuencia (SPS / PPS / VPS) en el Contenedor MP4
En los códecs de video modernos (H.264 / AVC y H.265 / HEVC), la reproducción depende de los conjuntos de parámetros de secuencia (*Sequence Parameter Set - SPS*) y de imagen (*Picture Parameter Set - PPS*). Estos parámetros definen:
- Perfil y Nivel del códec (ej. High Profile L4.1 vs High Profile L5.1).
- Modo de codificación de entropía (CABAC vs CAVLC).
- Cantidad máxima de fotogramas de referencia (*reference frames*).
- Tablas de cuantización y matrices de color.

**El error en el motor actual:**
1. **Slice 0 (Borde inicial recodificado):** Se codifica con `libx264` o `h264_nvenc`. FFmpeg genera un SPS y PPS nuevos basados en sus opciones por defecto.
2. **Slice 1 (Cuerpo copiado):** Proviene del archivo original (grabado por una cámara, teléfono, OBS o software de captura), y posee un SPS y PPS completamente distintos.
3. **Slice 2 (Borde final recodificado):** Se codifica nuevamente, generando otro conjunto de parámetros.
4. **Etapa de unión (`concat demuxer` con `-c copy`):**
   ```rust
   let mut args_concat = vec![
       "-f".to_string(), "concat".to_string(),
       "-safe".to_string(), "0".to_string(),
       "-i".to_string(), list_path_str,
       "-c".to_string(), "copy".to_string(),
   ];
   ```
   En el formato MP4 (`.mp4`), los metadatos de configuración del flujo de video se almacenan de manera global en la cabecera del archivo (átomo `avcC` o `hvcC` dentro de `moov`). El demuxer `concat` con `-c copy` **únicamente toma el SPS/PPS del primer archivo de la lista (`slice_0000.mp4`)**.
5. **Consecuencia en los reproductores:**
   - La GPU o decodificador del reproductor (VLC, Windows Media Player, QuickTime) se inicializa con el perfil y las tablas de `slice_0000`.
   - Tan pronto como la reproducción entra en `slice_0001`, el decodificador recibe paquetes NAL codificados con parámetros distintos, pero cree que siguen aplicando los de `slice_0000`.
   - **Resultado:** La imagen estalla en macrobloques descompuestos, artefactos verdes o pérdida total de sincronización.
   - **En Scrubbing / Búsqueda rápida:** Al saltar rápidamente en la barra de tiempo, el reproductor cae directamente en un fotograma del cuerpo copiado sin haber procesado los parámetros correctos, provocando corrupción visual severa hasta encontrar otro IDR con cabeceras globales compatibles o congelando la imagen por completo.

### 3.2. Confusión entre fotogramas Intra (I-frames) y fotogramas IDR (Open GOP)
En `engine.rs` (función `get_keyframes_in_range`, líneas 494-532):
```rust
let output = Command::new("ffprobe")
    .args([
        "-v", "error",
        "-select_streams", "v:0",
        "-skip_frame", "nokey",
        "-show_entries", "frame=pts_time",
        "-of", "csv=p=0",
        "-read_intervals", &range_str,
        path,
    ])
```
- La opción `-skip_frame nokey` le indica a `ffprobe` que liste fotogramas que contienen banderas de fotograma clave. En H.264/HEVC, esto devuelve tanto fotogramas **IDR** (GOP cerrado) como fotogramas **I no IDR** (GOP abierto / *recovery points*).
- En un **GOP abierto (Open GOP)** —muy común en grabaciones de smartphones, OBS y cámaras modernas para optimizar compresión—, los fotogramas B que siguen al fotograma I toman como referencia fotogramas P que estaban **antes** de dicho fotograma I.
- Al cortar en un I-frame de Open GOP con `-c copy`, los fotogramas de referencia previos han sido eliminados por el corte.
- **Resultado:** Los fotogramas B quedan "huérfanos". Al reproducir esa sección o hacer búsqueda rápida, el decodificador intenta calcular la imagen basándose en datos inexistentes, generando imágenes distorsionadas, cuadros grises y desgarro (*tearing*).

### 3.3. Discontinuidad de Estampas de Tiempo (PTS / DTS) y Destrucción de Índices de Búsqueda
1. **Diferencias de Timebase:**
   - El slice recodificado generado por FFmpeg adquiere típicamente un `time_base` predeterminado (por ejemplo, `1/12800`, `1/15360` o `1/90000`).
   - El slice copiado conserva el `time_base` original del archivo de entrada (que puede ser `1/1000`, `1/30000`, etc.).
   - Al unirlos con `concat` y `-c copy` hacia un contenedor `.mp4`, FFmpeg no unifica la escala de timestamps de los paquetes, provocando advertencias internas de tipo `Non-monotonous DTS in output stream`.
2. **Reordenamiento de fotogramas B (DTS vs PTS):**
   - En flujos con fotogramas B, el tiempo de decodificación (DTS) y el de presentación (PTS) difieren.
   - El uso de `-avoid_negative_ts make_zero` en los fragmentos copiados desplaza los timestamps de los primeros paquetes sin compensar adecuadamente el *edit list* (`elst`) del contenedor MP4.
   - Esto corrompe las tablas de búsqueda de fotogramas clave (`stss`) y de tiempo a muestra (`stts`, `ctts`) del archivo final.
   - **Efecto directo:** Cuando el usuario hace *scrubbing* rápido en VLC o MPV, el reproductor consulta estas tablas para saltar a un segundo específico; al estar corruptas o desfasadas, el salto aterriza en paquetes intermedios no decodificables, mostrando basura digital.

### 3.4. Inconsistencia Crítica en el Flujo de Audio
Obsérvese la disparidad entre cómo se procesa el audio en los slices:
- **Slice Recodificado (Líneas 96-103 en `engine.rs`):**
  ```rust
  v.extend([
      "-c:a".to_string(), "aac".to_string(),
      "-b:a".to_string(), "256k".to_string(),
      "-af".to_string(), "aresample=async=1:first_pts=0".to_string(),
  ]);
  ```
- **Slice Copiado (Línea 1108 en `engine.rs`):**
  ```rust
  s_cmd.extend([
      "-c".to_string(), "copy".to_string(),
  ]);
  ```
**¿Qué sucede al concatenar?**
1. Si el video original tenía audio en formato **AC3, Opus, MP3 o PCM**, el Slice 0 tiene audio AAC, mientras que el Slice 1 tiene el audio original.
2. Al concatenar con `-c copy`, el archivo MP4 resultante declara en su cabecera que la pista es AAC, pero a los pocos segundos empieza a recibir paquetes con una estructura de compresión y tamaño de trama completamente distintos.
3. Incluso si el original ya era AAC: el codificador de FFmpeg añade un retraso de inicialización (*priming delay* de 1024 muestras) y utiliza una tasa de 256 kbps, mientras que el flujo copiado tiene otra tasa y otro retraso.
4. **Resultado:** En cada punto de corte se escucha un clic/chasquido molesto, y en muchos reproductores el audio se desincroniza progresivamente o se silencia por completo tras el primer salto.

---

## 4. Análisis del Modo Reencode y de Clips Múltiples

En el modo `reencode` (Líneas 938-998):
- Cuando hay más de un clip, el motor actual también renderiza cada clip individualmente como `chunk_0000.mp4`, `chunk_0001.mp4`, etc., en una carpeta temporal, y luego los une con `concat` demuxer y `-c copy`.
- Aunque en este caso los códecs de video coinciden entre sí (porque todos fueron codificados por FFmpeg con la misma configuración), los cortes de audio en límites de paquetes AAC (bloques fijos de 1024 muestras $\approx 21.3$ ms a 48 kHz) provocan que la duración del audio de cada chunk difiera levemente de la duración del video (por ejemplo, video de 5.000 s y audio de 5.013 s).
- Al concatenar múltiples clips con `-c copy`, este desfase se acumula: tras 4 o 5 cortes, el audio y el video pueden quedar desfasados en más de 100 ms.

---

## 5. Soluciones Recomendadas y Hoja de Ruta

Para solucionar de forma definitiva tanto la falta de precisión milimétrica como las corrupciones de video y audio, se proponen las siguientes acciones correctivas:

### Fase 1: Corrección de Precisión en Slices (Inmediata)
1. **Eliminar el umbral restrictivo de 80 ms:**
   - La decisión de recodificar un borde debe basarse en la distancia real en fotogramas (`> 0.5 / fps`), no en una constante fija de 80 ms.
   - Si el punto de corte no coincide exactamente con el timestamp del keyframe, debe recodificarse el fragmento de borde sin omitirlo.
2. **Eliminar la duplicación de fotogramas en fronteras:**
   - Calcular la duración del borde inicial de forma que termine exactamente en el fotograma previo a `kf` (usando `-vframes` o limitando la duración a `(kf - start) - (0.5 / fps)`).
   - Para el slice de cuerpo copiado, asegurar que el seek no incluya el keyframe final si este ya será el punto de partida del borde final recodificado.

### Fase 2: Eliminación de Corrupciones de Video y Audio (Bitstream Integrity)
1. **Unificación de Parámetros de Codificación de Bordes:**
   - En `build_smart_slice_encoder_args`, clonar explícitamente los parámetros del video de origen:
     - `profile:v` (ej. `high`, `main`).
     - `level` (ej. `4.1`, `5.1`).
     - `time_base` y framerate constante (`-r`).
   - Aplicar el filtro de flujo de bits:
     - Para H.264: `-bsf:v h264_mp4toannexb` en los fragmentos intermedios o forzar inserción de parámetros SPS/PPS repetidos (`-bsf:v dump_extra` o `-movflags +global_sps_pps`).
     - Para HEVC: `-bsf:v hevc_mp4toannexb`.
2. **Uso de Contenedores Intermedios Transparentes:**
   - En lugar de guardar los slices intermedios como `.mp4` (que sufre con cambios de parámetros y timebases), utilizar **MPEG-TS (`.ts`)** o **Matroska (`.mkv`)** como contenedores temporales durante el proceso de corte y unión. MPEG-TS y Matroska permiten de forma nativa la variación de SPS/PPS entre paquetes sin corromper el contenedor.
   - En la unión final, se convierte limpiamente al contenedor de salida solicitado por el usuario (`.mp4`, `.mkv`, etc.).
3. **Manejo Coherente del Audio:**
   - No mezclar audio recodificado con audio copiado mediante `-c copy` si el códec o los parámetros difieren.
   - Si el audio del video original es compatible y se busca velocidad, copiar la pista de audio continua o recodificar todo el audio a una tasa constante uniforme a lo largo de todos los slices para evitar fallos de decodificación.

### Fase 3: Detección Real de Keyframes IDR (Closed GOP)
- Modificar `get_keyframes_in_range` para que filtre exclusivamente fotogramas clave IDR / puntos de acceso aleatorio cerrados:
  ```bash
  ffprobe -v error -select_streams v:0 \
    -show_frames -show_entries frame=pkt_pts_time,key_frame,pict_type,flags \
    -read_intervals [intervalo] [archivo]
  ```
  Esto evita cortar en I-frames de Open GOP que dejen fotogramas B huérfanos.

### Fase 4: Modo "Fast GPU Render" como Alternativa Segura
- Con tarjetas NVIDIA modernas (NVENC detectado en el sistema), la velocidad de recodificación completa supera frecuentemente los **350 a 600 fps** (un video de 5 minutos se procesa en menos de 10 segundos).
- Ofrecer o activar un aviso cuando el video original presente características complejas (como Open GOP, framerate variable VFR o perfiles exóticos), recomendando la recodificación completa por GPU, la cual garantiza **cero corrupción, 100% de compatibilidad en todos los reproductores y precisión absoluta fotograma a fotograma**.

---

*Informe generado para revisión y planificación del equipo de desarrollo.*
