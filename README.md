<div align="center">

# 🎬 SmartCut

**Editor y cortador de video de alta precisión y velocidad extrema sin pérdida (Smart Cut).**  
*Desarrollado con Tauri v2, Vue 3, TypeScript, Rust y FFmpeg.*

[![Tauri v2](https://img.shields.io/badge/Tauri-v2.0-blue.svg?logo=tauri&logoColor=white)](https://tauri.app/)
[![Vue 3](https://img.shields.io/badge/Vue-3.5-4FC08D.svg?logo=vue.js&logoColor=white)](https://vuejs.org/)
[![Rust](https://img.shields.io/badge/Rust-2021%20edition-DEA584.svg?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Vite](https://img.shields.io/badge/Vite-8.0-646CFF.svg?logo=vite&logoColor=white)](https://vitejs.dev/)
[![FFmpeg](https://img.shields.io/badge/FFmpeg-4.4%2B%20%2F%206.x-007808.svg?logo=ffmpeg&logoColor=white)](https://ffmpeg.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-10b981.svg)](LICENSE)

</div>

---

## 💡 ¿Por qué SmartCut?

Al cortar video digital, tradicionalmente nos enfrentamos a dos extremos frustrantes:

1. **Corte directo sin pérdida (`-c copy`)**: Es casi instantáneo, pero está restringido a cortar exclusivamente en los **Keyframes (fotogramas I/IDR)**. Si recortas en una posición arbitraria, la reproducción sufre congelamientos de imagen, saltos de video o desincronización de audio.
2. **Re-codificación total (NLEs tradicionales)**: Permite cortar en cualquier fotograma exacto, pero fuerza a recomprimir todo el material. Esto degrada la calidad original, consume horas de CPU/GPU y desaprovecha espacio en disco.

### ⚡ La Solución: Algoritmo Smart Cut

**SmartCut combina lo mejor de ambos mundos:**
- Inspecciona la estructura de GOP (*Group of Pictures*) del video origen mediante `ffprobe`.
- **Re-codifica únicamente los márgenes milimétricos en los límites del corte:**
  - `HEAD`: Desde el punto de entrada (`IN`) hasta el primer keyframe.
  - `TAIL`: Desde el último keyframe hasta el punto de salida (`OUT`).
- **Copia el cuerpo central (`BODY`) bit a bit sin pérdida** (`-c copy`).
- Concatena las partes respetando de forma idéntica los parámetros originales de códec, perfil, espacio de color y muestreo de audio.

```text
Flujo de Video Original:
   |K0...........|K1......................|K2......................|K3...........
                 IN ^                                              ^ OUT

Segmentación Smart Cut:
                [ HEAD  ][              BODY (copy)              ][ TAIL ]
                 IN → K1   K1 ──────────────────────────────────→ K2   K2 → OUT
                re-encode           copia directa sin pérdida         re-encode
```

> **Resultado:** Cortes milimétricos al fotograma exacto generados en cuestión de segundos, conservando el **100% de la calidad original en más del 98% del video**.

---

## ✨ Características Principales

- 🎯 **Precisión al Fotograma (*Frame-Accurate*)**: Define puntos de entrada (`IN`) y salida (`OUT`) exactos sin importar la distancia entre keyframes.
- ⚡ **Aceleración por Hardware Automática**: Detección nativa de codificadores GPU para los fragmentos de borde:
  - **NVIDIA NVENC** (`h264_nvenc`, `hevc_nvenc`)
  - **Intel / AMD VAAPI** (`h264_vaapi`, `hevc_vaapi`)
  - **Apple Silicon VideoToolbox** (`h264_videotoolbox`, `hevc_videotoolbox`)
  - Fallback CPU multihilo optimizado (`libx264`, `libx265`).
- 🎞️ **Línea de Tiempo Cinematográfica**:
  - Manipulación multi-clip con auto-snap magnético.
  - Herramienta cuchilla (*Razor / Split*) con un solo toque (`C`).
  - Recorte interactivo de bordes (*Trim Handles*) con feedback visual.
  - Soporte de clips no secuenciales, reordenables y duplicados con preservación de sincronización A/V.
  - Zoom interactivo suave y regla temporal con formato de código de tiempo SMPTE (`HH:MM:SS:FF`).
- 🔊 **Visualización de Audio en Tiempo Real**:
  - Generación rápida de forma de onda (*waveform*).
  - Vúmetro estéreo de estudio (L/R) con lectura en decibelios (dBFS).
  - Modo de exportación de audio puro sin pérdida (AAC, MP3, FLAC).
- 🚀 **Streaming Interno HTTP 206 (Rust)**:
  - Servidor multihilo embebido en Rust con soporte para solicitudes parciales (`Range: bytes=...`).
  - Permite reproducir al instante archivos pesados (4K a 60 fps, HEVC, ProRes) sin saturar la memoria RAM.
- 🎨 **Interfaz de Usuario Studio Monochrome**:
  - Paleta profesional de alto contraste en tonos carbón y obsidiana con acentos sutiles en verde esmeralda.
  - Sin distracciones visuales, diseñada para largas sesiones de edición.

---

## ⌨️ Atajos de Teclado

| Atajo | Acción |
| :--- | :--- |
| <kbd>Espacio</kbd> | Reproducir / Pausar video |
| <kbd>J</kbd> / <kbd>K</kbd> / <kbd>L</kbd> | Retroceder / Pausar / Avanzar con velocidad incremental |
| <kbd>←</kbd> / <kbd>→</kbd> | Retroceder / Avanzar un fotograma exacto |
| <kbd>Shift</kbd> + <kbd>←</kbd> / <kbd>→</kbd> | Saltar 1 segundo |
| <kbd>I</kbd> | Establecer punto de entrada (**IN**) |
| <kbd>O</kbd> | Establecer punto de salida (**OUT**) |
| <kbd>C</kbd> o <kbd>X</kbd> | Dividir / Cortar clip en la posición del cabezal (*Split*) |
| <kbd>Supr</kbd> / <kbd>Backspace</kbd> | Eliminar clip seleccionado |
| <kbd>Ctrl</kbd> + <kbd>Z</kbd> | Deshacer (*Undo*) |
| <kbd>Ctrl</kbd> + <kbd>Y</kbd> / <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd> | Rehacer (*Redo*) |
| <kbd>Ctrl</kbd> + <kbd>E</kbd> | Abrir modal de exportación |
| <kbd>Esc</kbd> | Cerrar modales / Cancelar herramienta activa |

---

## 📦 Formatos y Códecs Soportados

| Tipo | Formatos / Códecs |
| :--- | :--- |
| **Contenedores de Video** | `.mp4`, `.mkv`, `.mov`, `.webm`, `.avi`, `.ts` |
| **Códecs de Video** | H.264 / AVC, H.265 / HEVC, VP9, VP8, AV1 (modo copy) |
| **Códecs de Audio** | AAC, MP3, Opus, FLAC, AC3, PCM / WAV |

---

## 📋 Requisitos del Sistema

### 1. Dependencias de Ejecución (Runtime)
- **FFmpeg y FFprobe** instalados y accesibles en el `PATH` del sistema:
  ```bash
  ffmpeg -version
  ffprobe -version
  ```

### 2. Linux (Códecs multimedia GStreamer para WebKitGTK)
En entornos Linux, el webview de Tauri utiliza GStreamer para la reproducción HTML5. Para habilitar códecs como H.264/HEVC/AAC, consulta la guía detallada en [DEPENDENCIAS.md](DEPENDENCIAS.md).

Ejemplo rápido en **Arch Linux**:
```bash
sudo pacman -Syu gst-plugins-good gst-libav gst-plugin-va
```

Ejemplo rápido en **Ubuntu / Debian**:
```bash
sudo apt update && sudo apt install -y ffmpeg gstreamer1.0-plugins-good gstreamer1.0-libav gstreamer1.0-vaapi
```

---

## 🚀 Instalación y Desarrollo

### Requisitos de Construcción
- **Node.js** (v18.x o superior) y **npm**.
- **Rust** (versión estable 1.75+ con `cargo`).
- Herramientas de compilación C/C++ estándar (`build-essential` en Debian/Ubuntu, `base-devel` en Arch).

### Pasos

1. **Clonar el repositorio:**
   ```bash
   git clone https://github.com/jiodev/SmartCut.git
   cd SmartCut
   ```

2. **Instalar dependencias del frontend:**
   ```bash
   npm install
   ```

3. **Iniciar en modo desarrollo:**
   Compila el backend en Rust y lanza el servidor Vite con recarga rápida (*hot-reload*):
   ```bash
   npm run tauri dev
   ```

4. **Ejecutar la suite de pruebas del backend (33 tests de integración):**
   ```bash
   cargo test --manifest-path src-tauri/Cargo.toml
   ```

5. **Compilar para producción:**
   ```bash
   npm run tauri build
   ```

---

## 🐧 Empaquetado para Arch Linux

El proyecto incluye soporte nativo para empaquetado en Arch Linux mediante `PKGBUILD`:

```bash
# Compilar binario y empaquetar paquete .pkg.tar.zst
npm run package:arch

# Instalar el paquete generado localmente
sudo pacman -U packaging/arch/smartcut-bin-1.0.0-1-x86_64.pkg.tar.zst
```

---

## 📁 Estructura del Proyecto

```text
SmartCut/
├── src/                          # Frontend (Vue 3 + TypeScript + Vite)
│   ├── components/
│   │   ├── common/               # Modales (About, Context Menu, etc.)
│   │   ├── export/               # Diálogo de exportación, selector de GPU y progreso
│   │   ├── header/               # Barra superior de proyecto e info de archivo
│   │   ├── player/               # Reproductor Cinema y estado vacío
│   │   ├── sidebar/              # Panel lateral de clips y vúmetro de audio
│   │   └── timeline/             # Regla, cabezal, toolbar, pistas y formas de onda
│   ├── composables/              # Lógica desacoplada (useTimeline, useVideoPlayer, useExport...)
│   ├── types/                    # Tipado de clips, segmentos de exportación y metadatos
│   ├── utils/                    # Formateo SMPTE de tiempo y funciones auxiliares
│   ├── App.vue                   # Contenedor principal de la interfaz
│   ├── main.js                   # Inicialización de Vue
│   └── style.css                 # Sistema de diseño global (Studio Monochrome + Emerald)
│
├── src-tauri/                    # Backend Nativo (Rust + Tauri v2)
│   ├── src/
│   │   ├── export/               # Motor Smart Cut:
│   │   │   ├── assemble.rs       # Concatenación y ensamblado milimétrico
│   │   │   ├── compatibility.rs  # Compatibilidad de códecs y contenedores
│   │   │   ├── encoder.rs        # Selección de parámetros de codificación
│   │   │   ├── engine.rs         # Orquestador del flujo de renderizado
│   │   │   ├── hardware.rs       # Detección de GPU (NVENC, VAAPI, VideoToolbox)
│   │   │   ├── planner.rs        # Segmentación matemática (HEAD / BODY / TAIL)
│   │   │   ├── probe.rs          # Análisis de GOP y metadatos de pistas
│   │   │   └── verify.rs         # Auditoría post-exportación y chequeo de integridad
│   │   ├── ffmpeg/               # Extracción de picos de audio, miniaturas y metadatos
│   │   ├── server/               # Servidor HTTP 206 multihilo para streaming
│   │   ├── lib.rs                # Comandos Tauri expuestos
│   │   └── main.rs               # Entrada principal de la aplicación Tauri
│   ├── Cargo.toml                # Dependencias de Rust
│   └── tauri.conf.json           # Configuración de Tauri v2
│
├── packaging/                    # Empaquetado nativo para distribuciones (Arch Linux)
├── DEPENDENCIAS.md               # Guía completa de instalación de códecs
├── LICENSE                       # Licencia MIT
└── README.md                     # Documentación principal
```

---

## 🤝 Contribuciones

Las contribuciones son bienvenidas. Si encuentras un caso especial con algún contenedor o códec inusual:

1. Crea un **Issue** adjuntando los metadatos obtenidos con `ffprobe -v quiet -print_format json -show_format -show_streams <tu_archivo>`.
2. Envía un **Pull Request** con la corrección o mejora.

---

## 📄 Licencia

Este proyecto está distribuido bajo la Licencia **MIT**. Consulta el archivo [LICENSE](LICENSE) para más información.
