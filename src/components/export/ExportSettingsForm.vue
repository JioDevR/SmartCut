<script setup lang="ts">
import { computed } from 'vue';
import type { HardwareEncoderInfo, ClipItem, VideoMetadata, ExportPlanSummary } from '../../types';
import { formatTimecode } from '../../utils/formatters';

const props = defineProps<{
  exportOutputPath: string;
  exportMode: 'smart_cut' | 'reencode' | 'individual_clips';
  exportScope?: 'selected' | 'timeline';
  selectedClip?: ClipItem | null;
  availableEncoders: HardwareEncoderInfo[];
  selectedEncoder: string;
  sortedClips: ClipItem[];
  metadata: VideoMetadata;
  containerFormat?: 'mp4' | 'mkv' | 'mov' | 'webm';
  crf?: number;
  preset?: 'ultrafast' | 'fast' | 'medium' | 'slow';
  audioOnly?: boolean;
  audioFormat?: 'mp3' | 'aac' | 'flac';
  planSummary?: ExportPlanSummary | null;
  exportErrorMessage?: string;
}>();

const emit = defineEmits<{
  (e: 'update:exportMode', mode: 'smart_cut' | 'reencode' | 'individual_clips'): void;
  (e: 'update:exportScope', scope: 'selected' | 'timeline'): void;
  (e: 'update:selectedEncoder', enc: string): void;
  (e: 'update:containerFormat', fmt: 'mp4' | 'mkv' | 'mov' | 'webm'): void;
  (e: 'update:crf', crf: number): void;
  (e: 'update:preset', preset: 'ultrafast' | 'fast' | 'medium' | 'slow'): void;
  (e: 'update:audioOnly', audioOnly: boolean): void;
  (e: 'update:audioFormat', fmt: 'mp3' | 'aac' | 'flac'): void;
  (e: 'browse-path'): void;
}>();

const totalDuration = computed(() => {
  return props.sortedClips.reduce((acc, c) => acc + (c.end - c.start), 0);
});

const outputExt = computed(() => {
  if (props.audioOnly) {
    return (props.audioFormat || 'mp3').toUpperCase();
  }
  return (props.containerFormat || 'mkv').toUpperCase();
});

const hasHardwareGpu = computed(() => {
  return props.availableEncoders.some((e) => e.id !== 'cpu');
});

const activeEncoderInfo = computed(() => {
  return props.availableEncoders.find((e) => e.id === props.selectedEncoder) || props.availableEncoders[0];
});

function getEncoderIcon(id: string): string {
  switch (id) {
    case 'nvenc':
      return 'bolt';
    case 'qsv':
      return 'settings';
    case 'amf':
    case 'vaapi':
    case 'videotoolbox':
      return 'bolt';
    case 'cpu':
      return 'developer_board';
    default:
      return 'memory';
  }
}
</script>

<template>
  <div class="export-modal-body">
    <!-- 1. MODO DE EXPORTACIÓN -->
    <div class="export-section">
      <label class="section-label">MODO DE EXPORTACIÓN</label>
      <div class="export-mode-cards">
        <!-- Smart Cut -->
        <div
          class="mode-card"
          :class="{ active: exportMode === 'smart_cut' }"
          @click="emit('update:exportMode', 'smart_cut')"
        >
          <div class="mode-card-header">
            <span class="material-symbols-outlined mode-icon text-green">bolt</span>
            <span class="mode-name">Smart Cut</span>
          </div>
          <p class="mode-desc">
            Renderiza únicamente los bordes de cada corte para exactitud milimétrica. El resto del video se copia sin pérdida en segundos.
          </p>
        </div>

        <!-- Clips como Archivos Separados -->
        <div
          class="mode-card"
          :class="{ active: exportMode === 'individual_clips' }"
          @click="emit('update:exportMode', 'individual_clips')"
        >
          <div class="mode-card-header">
            <span class="material-symbols-outlined mode-icon">play_circle</span>
            <span class="mode-name">Clips como Archivos Separados</span>
          </div>
          <p class="mode-desc">
            Exporta cada corte generado en la línea de tiempo como un archivo independiente.
          </p>
        </div>
      </div>
    </div>

    <!-- 2. ACELERACIÓN POR HARDWARE -->
    <div class="export-section" v-if="availableEncoders.length > 0">
      <div class="section-header-row">
        <label class="section-label">ACELERACIÓN POR HARDWARE</label>
        <div v-if="hasHardwareGpu" class="badge-gpu-detected">
          <span class="gpu-dot"></span>
          <span>GPU Detectada</span>
        </div>
        <div v-else class="badge-cpu-detected">
          <span class="cpu-dot"></span>
          <span>CPU Detectado</span>
        </div>
      </div>

      <div class="hardware-card">
        <!-- Estado de la GPU / Acelerador activo -->
        <div class="hardware-top-row">
          <div class="hardware-left">
            <div class="hardware-icon-box">
              <span class="material-symbols-outlined text-green">
                {{ activeEncoderInfo?.id === 'cpu' ? 'developer_board' : 'bolt' }}
              </span>
            </div>
            <div class="hardware-title-group">
              <span class="gpu-name-text">{{ activeEncoderInfo?.gpu_name || 'GPU del Sistema' }}</span>
              <span class="gpu-tag-text">{{ activeEncoderInfo?.short_tag || 'NVENC' }}</span>
            </div>
          </div>
          <span v-if="activeEncoderInfo?.is_recommended" class="badge-recommended">
            Recomendado
          </span>
        </div>

        <!-- Pestañas horizontales de selección de encoders disponibles -->
        <div class="encoder-tabs-row">
          <button
            v-for="enc in availableEncoders"
            :key="enc.id"
            type="button"
            class="encoder-tab-btn"
            :class="{ active: selectedEncoder === enc.id }"
            @click="emit('update:selectedEncoder', enc.id)"
          >
            <span class="material-symbols-outlined tab-icon">
              {{ getEncoderIcon(enc.id) }}
            </span>
            <span class="tab-label">{{ enc.name }}</span>
          </button>
        </div>
      </div>
    </div>

    <!-- 3. CONTENEDOR DE SALIDA -->
    <div class="export-section">
      <div class="section-header-row">
        <label class="section-label">CONTENEDOR DE SALIDA</label>
        <label class="audio-only-toggle">
          <input
            type="checkbox"
            :checked="audioOnly"
            @change="emit('update:audioOnly', ($event.target as HTMLInputElement).checked)"
          />
          <span class="toggle-text">Extraer solo audio</span>
        </label>
      </div>

      <!-- Selector de video si no es solo audio -->
      <div v-if="!audioOnly" class="format-pills">
        <button
          v-for="fmt in (['mkv', 'mp4', 'mov', 'webm'] as const)"
          :key="fmt"
          type="button"
          class="format-pill-btn"
          :class="{ active: (containerFormat || 'mkv') === fmt }"
          @click="emit('update:containerFormat', fmt)"
        >
          {{ fmt.toUpperCase() }}{{ fmt === 'mkv' ? ' (Recomendado)' : '' }}
        </button>
      </div>

      <!-- Selector de audio si es solo audio -->
      <div v-else class="format-pills">
        <button
          v-for="afmt in (['mp3', 'aac', 'flac'] as const)"
          :key="afmt"
          type="button"
          class="format-pill-btn audio-pill"
          :class="{ active: (audioFormat || 'mp3') === afmt }"
          @click="emit('update:audioFormat', afmt)"
        >
          {{ afmt.toUpperCase() }}{{ afmt === 'mp3' ? ' (Recomendado)' : '' }}
        </button>
      </div>
    </div>

    <!-- 4. UBICACIÓN Y NOMBRE DEL ARCHIVO -->
    <div class="export-section">
      <label class="section-label">UBICACIÓN Y NOMBRE DEL ARCHIVO</label>
      <div class="file-path-picker">
        <input
          type="text"
          class="input-file-path"
          :value="exportOutputPath"
          placeholder="Selecciona la ruta de guardado..."
          readonly
          :title="exportOutputPath"
        />
        <button type="button" class="btn-browse-file" @click="emit('browse-path')">
          <span class="material-symbols-outlined">folder</span>
          <span>Examinar...</span>
        </button>
      </div>
    </div>

    <!-- 5. RESUMEN DEL PROYECTO -->
    <div class="export-summary-box">
      <!-- Columna 1 -->
      <div class="summary-col">
        <div class="summary-item">
          <span class="summary-label">CLIPS A EXPORTAR</span>
          <span class="summary-val">{{ sortedClips.length }} clips</span>
        </div>
        <div class="summary-item">
          <span class="summary-label">CÓDEC</span>
          <span class="summary-val">{{ metadata.video_codec.toUpperCase() }}</span>
        </div>
      </div>

      <!-- Columna 2 -->
      <div class="summary-col">
        <div class="summary-item">
          <span class="summary-label">DURACIÓN RESULTANTE</span>
          <span class="summary-val">{{ formatTimecode(totalDuration, metadata.fps) }}</span>
        </div>
        <div class="summary-item">
          <span class="summary-label">FORMATO</span>
          <span class="summary-val">.{{ outputExt }}</span>
        </div>
      </div>

      <!-- Columna 3 -->
      <div class="summary-col">
        <div class="summary-item">
          <span class="summary-label">RESOLUCIÓN</span>
          <span class="summary-val">{{ metadata.width }}x{{ metadata.height }}</span>
        </div>
      </div>
    </div>

    <div v-if="exportErrorMessage" class="export-error-msg">
      <span class="material-symbols-outlined">error</span>
      <span>{{ exportErrorMessage }}</span>
    </div>
  </div>
</template>

<style scoped>
.export-modal-body {
  padding: 20px 24px;
  overflow-y: auto;
  max-height: 72vh;
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.export-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.section-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.section-header-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

/* 1. Modos de Exportación */
.export-mode-cards {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.mode-card {
  background-color: var(--surface-low);
  border: 1px solid var(--border-subtle);
  border-radius: 10px;
  padding: 14px 16px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.mode-card:hover {
  background-color: var(--surface-mid);
  border-color: var(--border-strong);
}

.mode-card.active {
  border: 1.5px solid var(--accent-primary);
  background-color: var(--accent-primary-dim);
  box-shadow: 0 0 14px var(--accent-primary-glow);
}

.mode-card-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}

.mode-icon {
  font-size: 18px;
  color: var(--text-muted);
}

.mode-icon.text-green {
  color: var(--accent-primary);
}

.mode-name {
  font-size: 13.5px;
  font-weight: 600;
  color: var(--text-pure);
}

.mode-desc {
  font-size: 11.5px;
  color: var(--text-secondary);
  line-height: 1.45;
  margin: 0;
  padding-left: 26px;
}

/* 2. Aceleración por Hardware */
.badge-gpu-detected {
  display: flex;
  align-items: center;
  gap: 6px;
  background-color: var(--accent-primary-dim);
  border: 1px solid rgba(16, 185, 129, 0.35);
  border-radius: 999px;
  padding: 2.5px 10px;
  font-size: 10.5px;
  font-weight: 600;
  color: var(--accent-primary-bright);
}

.gpu-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background-color: var(--accent-primary);
  box-shadow: 0 0 6px var(--accent-primary-bright);
}

.badge-cpu-detected {
  display: flex;
  align-items: center;
  gap: 6px;
  background-color: var(--surface-high);
  border: 1px solid var(--border-subtle);
  border-radius: 999px;
  padding: 2.5px 10px;
  font-size: 10.5px;
  font-weight: 600;
  color: var(--text-muted);
}

.cpu-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background-color: var(--text-muted);
}

.hardware-card {
  background-color: var(--surface-low);
  border: 1px solid var(--border-subtle);
  border-radius: 10px;
  padding: 14px 16px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.hardware-top-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.hardware-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.hardware-icon-box {
  width: 36px;
  height: 36px;
  border-radius: 6px;
  background: var(--accent-primary-dim);
  border: 1px solid rgba(16, 185, 129, 0.25);
  display: flex;
  align-items: center;
  justify-content: center;
}

.hardware-icon-box .material-symbols-outlined {
  font-size: 20px;
}

.hardware-title-group {
  display: flex;
  align-items: center;
  gap: 8px;
}

.gpu-name-text {
  font-size: 13.5px;
  font-weight: 700;
  color: var(--text-pure);
}

.gpu-tag-text {
  font-size: 11px;
  font-weight: 700;
  color: var(--accent-primary);
  font-family: var(--font-mono);
}

.badge-recommended {
  font-size: 10.5px;
  font-weight: 600;
  color: var(--accent-primary-bright);
  border: 1px solid rgba(16, 185, 129, 0.35);
  background-color: var(--accent-primary-dim);
  padding: 3px 10px;
  border-radius: 6px;
}

.encoder-tabs-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
  gap: 10px;
}

.encoder-tab-btn {
  background-color: var(--surface-dim);
  border: 1px solid var(--border-subtle);
  border-radius: 6px;
  color: var(--text-secondary);
  padding: 9px 12px;
  font-size: 11.5px;
  font-weight: 500;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.encoder-tab-btn:hover {
  background-color: var(--surface-high);
  color: var(--text-pure);
  border-color: var(--border-strong);
}

.encoder-tab-btn.active {
  border: 1.5px solid var(--accent-primary);
  background-color: var(--accent-primary-dim);
  color: var(--accent-primary-bright);
  font-weight: 600;
}

.tab-icon {
  font-size: 16px;
}

/* 3. Contenedor de Salida */
.audio-only-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  font-size: 11.5px;
  color: var(--text-secondary);
  font-weight: 600;
  user-select: none;
}

.audio-only-toggle input[type="checkbox"] {
  accent-color: var(--accent-primary);
  cursor: pointer;
  width: 14px;
  height: 14px;
}

.format-pills {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}

.format-pill-btn {
  background-color: var(--surface-low);
  border: 1px solid var(--border-subtle);
  border-radius: 8px;
  color: var(--text-secondary);
  font-family: var(--font-mono);
  font-size: 11.5px;
  font-weight: 600;
  padding: 8px 18px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.format-pill-btn:hover {
  background-color: var(--surface-mid);
  color: #ffffff;
  border-color: var(--border-strong);
}

.format-pill-btn.active {
  border: 1.5px solid var(--accent-primary);
  background-color: var(--accent-primary-dim);
  color: var(--accent-primary-bright);
}

.format-pill-btn.audio-pill.active {
  border-color: var(--accent-primary);
  background-color: var(--accent-primary-dim);
  color: var(--accent-primary-bright);
}

/* 4. Ubicación y Nombre del Archivo */
.file-path-picker {
  display: flex;
  gap: 10px;
  align-items: stretch;
}

.input-file-path {
  flex: 1;
  background-color: var(--surface-dim);
  border: 1px solid var(--border-subtle);
  border-radius: 8px;
  padding: 9px 14px;
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-size: 11.5px;
  outline: none;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.btn-browse-file {
  display: flex;
  align-items: center;
  gap: 8px;
  background-color: var(--surface-high);
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  color: var(--text-primary);
  padding: 0 16px;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
  white-space: nowrap;
}

.btn-browse-file:hover {
  background-color: var(--surface-highest);
  border-color: var(--border-strong);
}

.btn-browse-file .material-symbols-outlined {
  font-size: 16px;
}

/* 5. Resumen del Proyecto */
.export-summary-box {
  background-color: var(--surface-low);
  border: 1px solid var(--border-subtle);
  border-radius: 10px;
  padding: 16px 20px;
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 16px 20px;
}

.summary-col {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.summary-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.summary-label {
  font-size: 10px;
  font-weight: 700;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.summary-val {
  font-family: var(--font-mono);
  font-size: 13.5px;
  font-weight: 700;
  color: #ffffff;
}

.export-error-msg {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #ef4444;
  font-size: 12px;
  background-color: rgba(239, 68, 68, 0.1);
  padding: 8px 12px;
  border-radius: 6px;
  border: 1px solid rgba(239, 68, 68, 0.3);
}
</style>
