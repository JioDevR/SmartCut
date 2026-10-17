<div align="center">

# 🎬 SmartCut

**Cortador de video inteligente sin pérdida con precisión de frame (Smart Cut)**  
*Desarrollado con Tauri v2, Vue 3, TypeScript, Rust y FFmpeg.*

[![Status](https://img.shields.io/badge/status-etapa%20preliminar%20%2F%20alpha-orange.svg)](#-estado-del-proyecto)
[![Tauri](https://img.shields.io/badge/Tauri-v2-blue.svg?logo=tauri)](https://tauri.app/)
[![Vue 3](https://img.shields.io/badge/Vue-3.5-4FC08D.svg?logo=vue.js)](https://vuejs.org/)
[![Rust](https://img.shields.io/badge/Rust-2021%20edition-DEA584.svg?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

</div>

---

> [!WARNING]
> ### ⚠️ Estado del Proyecto: Etapa Preliminar (Alpha / WIP)
> **SmartCut se encuentra en una fase preliminar y experimental de desarrollo activo.**
>
> Aunque las funcionalidades esenciales de edición en línea de tiempo, análisis de GOP/keyframes y el motor de exportación Smart Cut están operativas, el software puede contener fallos, discrepancias menores de sincronización en ciertos contenedores o cambios arquitectónicos frecuentes.
>
> **No se recomienda aún para entornos de producción críticos.** ¡El feedback, reporte de issues y pruebas con diferentes formatos de video son sumamente bienvenidos!

---

## 💡 ¿Qué es SmartCut?

Al cortar video existen tradicionalmente dos alternativas:
1. **Corte sin pérdida directo (`-c copy`)**: Extremadamente rápido, pero limitado a cortar únicamente en **Keyframes (fotogramas I/IDR)**. Si intentas cortar en un segundo exacto que no coincide con un fotograma clave, el video salta, se congela o desincroniza.
2. **Re-codificación completa**: Permite precisión al frame exacto, pero requiere re-comprimir todo el material, lo que degrada la calidad original y toma un tiempo considerable.

**SmartCut resuelve esto mediante el algoritmo Smart Cut:**
- Identifica la estructura GOP (Group of Pictures) del archivo original mediante `ffprobe`.
- **Re-codifica únicamente los fragmentos milimétricos en los extremos del corte** (`HEAD` desde el punto de entrada hasta el primer keyframe; `TAIL` desde el último keyframe hasta el punto de salida).
- **Copia el cuerpo principal (`BODY`) bit a bit sin pérdida** (`-c copy`).
- Concatena las secciones manteniendo los mismos parámetros de códec, perfil, espacio de color y audio, logrando **cortes al fotograma exacto en cuestión de segundos y conservando el 100% de la calidad original en el 99% del video**.

```
Fuente:  |K0.......|K1.........|K2.........|K3.........|K4......
                 IN ^                                ^ OUT
Salida:         [ HEAD  ][        BODY (copy)       ][ TAIL ]
                 IN → K1   K1 ───────────────────→ K3   K3 → OUT
                re-encode        copia sin pérdida      re-encode
```

---

## ✨ Características Principales

- 🎯 **Precisión de Frame**: Define puntos de entrada (`IN`) y salida (`OUT`) exactos sin preocuparte por dónde caen los keyframes.
- ⚡ **Exportación Ultrarrápida**: Hasta 50x más rápido que renderizar el video completo en un editor tradicional.
- 🎞️ **Timeline Interactiva**:
  - Zoom, scroll continuo y regla temporal en tiempo real.
  - Multi-clip con soporte de reordenamiento y arrastre.
  - Ajuste de bordes (trim) visual e intuitivo.
- 🔊 **Visualización de Audio**: Generación de miniaturas y visualización de waveform.
- 🖥️ **Streaming Local de Baja Latencia**: Servidor HTTP interno en Rust para una reproducción fluida dentro de WebKit/WebView sin saturar memoria.
- 🎨 **Interfaz Moderna**: Tema oscuro optimizado, minimalista y reactivo.

---

## 📋 Requisitos del Sistema

### Dependencias de Ejecución (Runtime)
1. **FFmpeg y FFprobe** instalados y accesibles en el `PATH` del sistema:
   ```bash
   ffmpeg -version
   ffprobe -version
   ```
2. **Linux (Códecs multimedia GStreamer para WebKitGTK)**:
   En sistemas Linux, el webview de Tauri utiliza GStreamer para la reproducción HTML5 de video. Consulta la guía detallada en [DEPENDENCIAS.md](DEPENDENCIAS.md) para instalar los plugins necesarios (`gst-plugins-good`, `gst-libav`, etc.) según tu distribución (Arch, Ubuntu/Debian, Fedora).

---

## 🚀 Instalación y Desarrollo Local

### 1. Clonar el repositorio
```bash
git clone https://github.com/<TU_USUARIO>/SmartCut.git
cd SmartCut
```

### 2. Instalar dependencias del frontend
```bash
npm install
```

### 3. Ejecutar en modo desarrollo
Inicia el entorno de desarrollo con Vite + Tauri v2:
```bash
npm run tauri dev
```

### 4. Compilar para producción
Genera el binario ejecutable y paquetes nativos del sistema:
```bash
npm run tauri build
```

### 5. Ejecutar Tests Unitarios e Integración (Rust)
Para validar el motor de exportación y la lógica de GOP:
```bash
cd src-tauri
cargo test
```

---

## 📁 Estructura del Proyecto

```text
SmartCut/
├── src/                          # Frontend (Vue 3 + TypeScript)
│   ├── components/               # Componentes UI (Timeline, Player, Export, etc.)
│   ├── composables/              # Lógica de estado reactivo (useVideoPlayer, useTimeline, etc.)
│   ├── types/                    # Definiciones TypeScript de clips y modelos
│   ├── utils/                    # EDL, formateadores de tiempo y helpers
│   ├── App.vue                   # Vista principal
│   └── main.js                   # Punto de entrada de la aplicación Vue
│
├── src-tauri/                    # Backend Nativo (Rust + Tauri v2)
│   ├── src/
│   │   ├── export/               # Motor Smart Cut (planner, encoder, assemble, packets)
│   │   ├── ffmpeg/               # Extracción de metadatos, keyframes y audio
│   │   ├── server/               # Servidor HTTP local para streaming de video
│   │   ├── lib.rs                # Comandos Tauri expuestos al frontend
│   │   └── main.rs               # Entrada de la aplicación Tauri
│   ├── Cargo.toml                # Dependencias Rust (Tauri 2, serde, rfd, etc.)
│   └── tauri.conf.json           # Configuración de Tauri v2
│
├── packaging/                    # Empaquetado para distribuciones (Arch PKGBUILD, etc.)
├── RECURSOS/                     # Muestras de prueba para tests de integración
├── DEPENDENCIAS.md               # Guía completa de configuración de GStreamer/Linux
├── exportacion.md                # Especificación técnica del algoritmo Smart Cut
├── informeV2.md                  # Informe de trazabilidad y sincronización A/V
└── README.md                     # Documentación general
```

---

## 🗺️ Hoja de Ruta (Roadmap)

- [x] Arquitectura base con Tauri v2 y Vue 3.
- [x] Detección de GOPs y cálculo de keyframes mediante `ffprobe`.
- [x] Motor de exportación Smart Cut (`HEAD` re-encode + `BODY` copy + `TAIL` re-encode).
- [ ] Pulido final de sincronización de audio multi-clip (desfase por empalmes mixtos).
- [ ] Detección por hardware acelerado (NVENC, VAAPI, QSV) para los segmentos de re-encode.
- [ ] Exportación por lotes (Batch Export).
- [ ] Soporte para pistas de subtítulos incrustadas y múltiples canales de audio.
- [ ] Integración de binarios portables de FFmpeg / FFprobe para Windows y macOS.

---

## 🤝 Contribuciones y Feedback

¡Toda contribución es apreciada! Si encuentras un bug, tienes un caso de prueba con un códec específico o deseas proponer una mejora:

1. Abre un **Issue** describiendo el comportamiento y el formato de video utilizado (`ffprobe`).
2. Crea un **Pull Request** con tus cambios propuestos.

---

## 📄 Licencia

Este proyecto está bajo la Licencia **MIT**. Consulta el archivo [LICENSE](LICENSE) para más detalles.
