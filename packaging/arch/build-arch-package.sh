#!/usr/bin/env bash
set -e

# Directorio raíz del proyecto
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ARCH_DIR="${ROOT_DIR}/packaging/arch"

echo "==> 1. Compilando aplicación nativa para producción (Tauri con assets embebidos)..."
cd "${ROOT_DIR}"
npx tauri build --no-bundle

echo "==> 3. Preparando archivos del paquete en packaging/arch/..."
cp "${ROOT_DIR}/src-tauri/target/release/smartcut" "${ARCH_DIR}/smartcut"
chmod +x "${ARCH_DIR}/smartcut"

# Sincronizar iconos actualizados
cp "${ROOT_DIR}/src-tauri/icons/32x32.png" "${ARCH_DIR}/32x32.png"
cp "${ROOT_DIR}/src-tauri/icons/128x128.png" "${ARCH_DIR}/128x128.png"
cp "${ROOT_DIR}/src-tauri/icons/128x128@2x.png" "${ARCH_DIR}/128x128@2x.png"

echo "==> 4. Generando paquete para Arch Linux con makepkg..."
cd "${ARCH_DIR}"
rm -rf pkg src
makepkg -f

echo "==> ¡Proceso finalizado con éxito!"
echo "Paquete generado:"
ls -lh "${ARCH_DIR}"/*.pkg.tar.zst
