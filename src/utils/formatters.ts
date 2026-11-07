/**
 * Formatea segundos a código de tiempo con frames (HH:MM:SS:FF o MM:SS:FF)
 */
export function formatTimecode(secs: number, fps: number = 30): string {
  if (isNaN(secs) || secs < 0) secs = 0;
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = Math.floor(secs % 60);
  const f = Math.floor((secs % 1) * (fps || 30));
  const pad = (n: number, len: number = 2) => String(n).padStart(len, '0');

  if (h > 0) {
    return `${pad(h)}:${pad(m)}:${pad(s)}:${pad(f)}`;
  }
  return `${pad(m)}:${pad(s)}:${pad(f)}`;
}

/**
 * Formatea marcas de la regla según el nivel de zoom
 */
export function formatRulerTime(secs: number, level: number): string {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = Math.floor(secs % 60);
  const ms = Math.floor((secs % 1) * 100);
  const pad = (n: number, len: number = 2) => String(n).padStart(len, '0');

  if (level === 4) {
    return `${pad(m)}:${pad(s)}.${pad(ms)}`;
  } else if (level === 3) {
    return `${pad(m)}:${pad(s)}.${Math.floor(ms / 10)}`;
  } else if (h > 0) {
    return `${pad(h)}:${pad(m)}:${pad(s)}`;
  } else {
    return `${pad(m)}:${pad(s)}`;
  }
}

/**
 * Formatea tamaño en bytes a string legible (KB, MB, GB)
 */
export function formatBytes(bytes: number): string {
  if (!bytes) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
}
