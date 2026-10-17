# Dependencias del Sistema - SmartCut

Guía completa de librerías y dependencias necesarias para la correcta reproducción, renderizado y ejecución de **SmartCut** en Linux, Windows y macOS.

---

## 1. Instalación Rápida por Distribución (Linux)

En Linux, el motor webview de Tauri (`WebKitGTK`) delega la reproducción del reproductor de video HTML5 a **GStreamer**. Para reproducir sin errores contenedores `.mp4`, `.mkv`, `.mov` y códecs como **H.264**, **HEVC/H.265**, **AAC** y **VP9**, ejecuta el comando correspondiente a tu distribución:

### Arch Linux / CachyOS / Manjaro / EndeavourOS

> [!IMPORTANT]
> **Si te aparece el error:** `breaks dependency ... required by gst-plugin-gtk` o similar, se debe a una discrepancia de versión entre los repositorios remotos y tus paquetes locales (actualización parcial).

Elige **una de las dos opciones** para instalarlo sin conflictos:

#### Opción 1: Actualizar e instalar (Recomendada en Arch Linux)
Sincroniza los repositorios y actualiza los paquetes multimedia al mismo nivel de versión:
```bash
sudo pacman -Syu gst-plugins-good gst-libav gst-plugin-va
```

#### Opción 2: Actualizar únicamente el conjunto de GStreamer (Sin actualizar todo el sistema)
Si no deseas actualizar todo tu sistema en este momento, puedes actualizar en bloque todos los módulos de GStreamer juntos:
```bash
sudo pacman -S \
    gst-plugins-good \
    gst-plugins-base \
    gst-plugins-base-libs \
    gst-plugins-bad \
    gst-plugins-bad-libs \
    gst-plugins-ugly \
    gstreamer \
    gst-libav \
    gst-plugin-gtk \
    gst-plugin-va
```
*(Al listar todo el conjunto de GStreamer en el mismo comando, pacman actualiza todas las dependencias cruzadas a la misma versión y no habrá ningún conflicto).*

---

### Ubuntu / Debian / Linux Mint / Pop!_OS
```bash
sudo apt update && sudo apt install -y \
    ffmpeg \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-plugins-ugly \
    gstreamer1.0-libav \
    gstreamer1.0-vaapi
```

---

### Fedora / RHEL / AlmaLinux
```bash
sudo dnf install -y \
    ffmpeg \
    gstreamer1-plugins-base \
    gstreamer1-plugins-good \
    gstreamer1-plugins-bad-free \
    gstreamer1-plugins-ugly-free \
    gstreamer1-libav \
    gstreamer1-vaapi
```
*(Si usas repositorios RPM Fusion, puedes instalar `gstreamer1-plugins-bad-freeworld` y `gstreamer1-plugins-ugly` para soporte completo de todos los códecs privativos).*

---

## 2. Desglose y Función de Cada Dependencia

| Paquete | Obligatorio / Opcional | Función en SmartCut |
| :--- | :---: | :--- |
| **`gst-plugins-good`** | **Obligatorio** | Provee **`qtdemux`** (lector de MP4/MOV) y **`matroskademux`** (lector de MKV). Sin este paquete, el reproductor de video arroja error `4: Formato o códec incompatible`. |
| **`ffmpeg`** | **Obligatorio** | Motor CLI del backend de Rust. Realiza el análisis de metadatos (`ffprobe`), extracción paralela de fotogramas, generación de ondas espectrales y el corte sin pérdidas (*Smart Cut / Lossless*). |
| **`gst-libav`** | **Recomendado** | Conecta GStreamer con las librerías de FFmpeg para permitir decodificar códecs avanzados como **HEVC (H.265)** en el reproductor. |
| **`gst-plugins-bad`** | Recomendado | Soporte para parsers H.264/H.265 y formatos modernos de transmisión y contenedores. |
| **`gst-plugins-ugly`** | Recomendado | Códecs con restricciones de patente o comerciales (XviD, MP3 legacy, etc.). |
| **`gst-plugin-va`** | Opcional | Aceleración por hardware por GPU integrada Intel/AMD para desahogar la CPU durante la reproducción. |
| **`webkit2gtk-4.1`** | **Obligatorio** | Motor de renderizado visual y ventana de la aplicación de escritorio en Linux. |

---

## 3. ¿Cómo Comprobar que Todo Está Bien Instalado?

Puedes verificar en cualquier momento que los módulos de GStreamer estén disponibles ejecutando:

```bash
# 1. Verificar demuxer de MKV:
gst-inspect-1.0 matroskademux

# 2. Verificar demuxer de MP4/MOV:
gst-inspect-1.0 qtdemux

# 3. Verificar FFmpeg:
ffmpeg -version
```

Si ambos comandos devuelven la ficha técnica del plugin (sin decir `No such element`), tu sistema está 100% listo para reproducir cualquier video de forma nativa.

---

## 4. Instalación Nativa en Arch Linux / CachyOS (Recomendada)

En lugar de utilizar paquetes AppImage (que aíslan el entorno y pueden bloquear los plugins de GStreamer del sistema), SmartCut cuenta con un paquete nativo optimizado para Arch Linux y CachyOS:

### Instalar el paquete generado:
```bash
sudo pacman -U packaging/arch/smartcut-bin-0.1.0-1-x86_64.pkg.tar.zst
```

### O regenerar el paquete desde el PKGBUILD:
```bash
cd packaging/arch
makepkg -f
sudo pacman -U smartcut-bin-*.pkg.tar.zst
```

#### Ventajas del paquete nativo:
* **Integración total:** Ejecuta el binario 100% nativo (`/usr/bin/smartcut`) con acceso directo a todos los códecs de Pacman.
* **Ultraligero:** Pesa apenas ~2.2 MB frente a los más de 100 MB de un AppImage.
* **Acceso de escritorio:** Agrega automáticamente el lanzador e iconos en alta resolución en tu menú de aplicaciones (KDE, GNOME, etc.).

---

## 5. Dependencias para Compilar desde el Código Fuente (Desarrolladores)

Si vas a compilar el proyecto (`npm run tauri build` o `cargo build`):

### En Arch Linux:
```bash
sudo pacman -S --needed base-devel nodejs npm rust cargo webkit2gtk-4.1
```

### En Ubuntu / Debian:
```bash
sudo apt install -y build-essential curl libwebkit2gtk-4.1-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

---

## 6. Plataformas Windows y macOS

* **Windows:**
  * No requiere instalar GStreamer.
  * Utiliza el motor **WebView2** (basado en Edge) y **Windows Media Foundation** de forma nativa.
  * Solo requiere tener `ffmpeg` en el PATH del sistema o empaquetado junto al ejecutable.
* **macOS:**
  * No requiere librerías externas para reproducción; utiliza **AVFoundation** de forma nativa.
  * Solo requiere `ffmpeg` instalado (vía `brew install ffmpeg`).
