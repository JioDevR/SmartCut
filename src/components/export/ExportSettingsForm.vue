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
  exportErrorMessage: string;
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
  const parts = props.exportOutputPath.split('.');
  return parts.length > 1 ? parts.pop()?.toUpperCase() : 'MKV';
});
</script>

<template>
  <div class="export-modal-body">
    <!-- 0. Ámbito de Exportación -->
    <div class="export-section" v-if="sortedClips.length > 1">
      <label class="section-label">¿Qué deseas exportar?</label>
      <div class="export-scope-cards">
        <div
          v-if="selectedClip"
          class="scope-card"
          :class="{ active: (exportScope || 'selected') === 'selected' }"
          @click="emit('update:exportScope', 'selected')"
        >
          <div class="scope-header">
            <span class="material-symbols-outlined text-cyan">crop</span>
            <span class="scope-title">Clip Seleccionado</span>
            <span class="mode-badge-rec">Recomendado</span>
          </div>
          <p class="scope-desc">
            Exporta únicamente <strong>{{ selectedClip.name }}</strong> ({{ formatTimecode(selectedClip.start) }} - {{ formatTimecode(selectedClip.end) }}).
          </p>
          <div class="scope-meta">
            Duración: <span class="text-cyan font-mono">{{ (selectedClip.end - selectedClip.start).toFixed(2) }}s</span>
          </div>
        </div>

        <div
          class="scope-card"
          :class="{ active: exportScope === 'timeline' }"
          @click="emit('update:exportScope', 'timeline')"
        >
          <div class="scope-header">
            <span class="material-symbols-outlined text-amber">view_timeline</span>
            <span class="scope-title">Línea de Tiempo Completa</span>
          </div>
          <p class="scope-desc">
            Une todos los <strong>{{ sortedClips.length }} clips</strong> de la línea de tiempo en un solo video.
          </p>
          <div class="scope-meta">
            Duración total: <span class="text-amber font-mono">{{ totalDuration.toFixed(2) }}s</span>
          </div>
        </div>
      </div>
    </div>

    <!-- 1. Modo de Exportación -->
    <div class="export-section">
      <label class="section-label">Modo de Exportación</label>
      <div class="export-mode-cards">
        <!-- Smart Cut -->
        <div
          class="mode-card"
          :class="{ active: exportMode === 'smart_cut' }"
          @click="emit('update:exportMode', 'smart_cut')"
        >
          <div class="mode-card-header">
            <span class="material-symbols-outlined mode-icon text-cyan">bolt</span>
            <span class="mode-name">Smart Cut (Corte de Precisión)</span>
            <span class="mode-badge-rec">Recomendado</span>
          </div>
          <p class="mode-desc">
            Renderiza únicamente los bordes de cada corte para exactitud milimétrica. El resto del video se copia sin pérdida en segundos.
          </p>
        </div>

        <!-- Reencode -->
        <div
          class="mode-card"
          :class="{ active: exportMode === 'reencode' }"
          @click="emit('update:exportMode', 'reencode')"
        >
          <div class="mode-card-header">
            <span class="material-symbols-outlined mode-icon text-amber">movie_edit</span>
            <span class="mode-name">Renderizado Completo</span>
          </div>
          <p class="mode-desc">
            Recodifica el 100% del video de principio a fin usando aceleración por GPU.
          </p>
        </div>

        <!-- Clips Separados -->
        <div
          class="mode-card"
          :class="{ active: exportMode === 'individual_clips' }"
          @click="emit('update:exportMode', 'individual_clips')"
        >
          <div class="mode-card-header">
            <span class="material-symbols-outlined mode-icon text-emerald">video_library</span>
            <span class="mode-name">Clips como Archivos Separados</span>
          </div>
          <p class="mode-desc">
            Exporta cada corte generado en la línea de tiempo como un archivo independiente.
          </p>
        </div>
      </div>

      <!-- Resumen del Plan Smart Cut -->
      <div v-if="exportMode === 'smart_cut' && planSummary" class="smartcut-plan-box">
        <div class="smartcut-plan-header">
          <span class="material-symbols-outlined text-cyan">bolt</span>
          <span class="plan-title">Optimización Smart Cut Calculada</span>
        </div>
        <div class="plan-stats-grid">
          <div class="plan-stat">
            <span class="stat-num text-emerald">{{ planSummary.copy_duration.toFixed(1) }}s</span>
            <span class="stat-label">Copia directa (sin pérdida)</span>
          </div>
          <div class="plan-stat">
            <span class="stat-num text-amber">{{ planSummary.reencode_duration.toFixed(1) }}s</span>
            <span class="stat-label">Bordes de precisión (render)</span>
          </div>
        </div>
        <div class="smartcut-guarantee">
          <span class="material-symbols-outlined text-emerald">verified</span>
          <span>Formato original (.{{ outputExt }}), códec ({{ metadata.video_codec.toUpperCase() }}), {{ metadata.fps }} FPS y {{ metadata.width }}x{{ metadata.height }} se preservan exactamente iguales.</span>
        </div>
      </div>
    </div>

    <!-- 2. Dispositivo / Acelerador -->
    <div class="export-section" v-if="availableEncoders.length > 0">
      <label class="section-label">Aceleración por Hardware (GPU Detectada)</label>
      <div class="encoder-cards-list">
        <div
          v-for="enc in availableEncoders"
          :key="enc.id"
          class="encoder-card"
          :class="{ active: selectedEncoder === enc.id }"
          @click="emit('update:selectedEncoder', enc.id)"
        >
          <div class="encoder-header">
            <span class="material-symbols-outlined encoder-icon" :class="{
              'text-cyan': enc.id === 'nvenc',
              'text-sky': enc.id === 'qsv',
              'text-amber': enc.id === 'cpu'
            }">
              {{ enc.id === 'nvenc' ? 'bolt' : (enc.id === 'qsv' ? 'memory' : 'developer_board') }}
            </span>
            <span class="encoder-name">{{ enc.name }}</span>
            <span v-if="enc.is_recommended" class="mode-badge-rec">Detectado</span>
          </div>
          <p class="encoder-desc">{{ enc.description }}</p>
        </div>
      </div>
    </div>

    <!-- 3. Formato de Contenedor y Modo Audio -->
    <div class="export-section">
      <div class="section-label-row">
        <label class="section-label">Contenedor de Salida</label>
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
          {{ fmt.toUpperCase() }} {{ fmt === 'mkv' ? '(Recomendado)' : '' }}
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
          {{ afmt.toUpperCase() }}
        </button>
      </div>
    </div>

    <!-- 4. Calidad y Velocidad (Ajustes Pro) -->
    <div class="export-section" v-if="!audioOnly && exportMode !== 'smart_cut'">
      <div class="crf-header-row">
        <label class="section-label">Calidad de Video (CRF: {{ crf ?? 20 }})</label>
        <span class="crf-desc-badge">
          {{ (crf ?? 20) <= 18 ? 'Ultra Calidad (Mayor tamaño)' : ((crf ?? 20) <= 22 ? 'Balance Óptimo' : 'Alta Compresión') }}
        </span>
      </div>
      <div class="slider-with-labels">
        <input
          type="range"
          min="16"
          max="28"
          step="1"
          class="crf-slider"
          :value="crf ?? 20"
          @input="emit('update:crf', Number(($event.target as HTMLInputElement).value))"
        />
        <div class="slider-ticks-row">
          <span>16 (Sin Pérdida Vis.)</span>
          <span>20 (Recomendado)</span>
          <span>28 (Archivo Pequeño)</span>
        </div>
      </div>

      <div class="preset-section">
        <label class="sub-section-label">Velocidad de Codificación (Preset)</label>
        <div class="preset-pills">
          <button
            v-for="p in ([
              { id: 'ultrafast', label: 'Ultra Rápido' },
              { id: 'fast', label: 'Rápido' },
              { id: 'medium', label: 'Equilibrado' },
              { id: 'slow', label: 'Lento (Mejor calidad)' }
            ] as const)"
            :key="p.id"
            type="button"
            class="preset-pill-btn"
            :class="{ active: (preset || 'fast') === p.id }"
            @click="emit('update:preset', p.id)"
          >
            {{ p.label }}
          </button>
        </div>
      </div>
    </div>

    <!-- 5. Destino de Archivo -->
    <div class="export-section">
      <label class="section-label">Ubicación y Nombre del Archivo</label>
      <div class="file-path-picker">
        <input
          type="text"
          class="input-file-path"
          :value="exportOutputPath"
          placeholder="Selecciona la ruta de guardado..."
          readonly
        />
        <button type="button" class="btn-browse-file" @click="emit('browse-path')">
          <span class="material-symbols-outlined">folder_open</span>
          <span>Examinar...</span>
        </button>
      </div>
    </div>

    <!-- Resumen del proyecto -->
    <div class="export-summary-box">
      <div class="summary-item">
        <span class="summary-label">Clips a exportar</span>
        <span class="summary-val">{{ sortedClips.length }} clips</span>
      </div>
      <div class="summary-item">
        <span class="summary-label">Duración resultante</span>
        <span class="summary-val text-cyan">{{ formatTimecode(totalDuration, metadata.fps) }}</span>
      </div>
      <div class="summary-item">
        <span class="summary-label">Resolución</span>
        <span class="summary-val">{{ metadata.width }}x{{ metadata.height }}</span>
      </div>
      <div class="summary-item">
        <span class="summary-label">Códec</span>
        <span class="summary-val">{{ metadata.video_codec.toUpperCase() }}</span>
      </div>
      <div class="summary-item">
        <span class="summary-label">Formato</span>
        <span class="summary-val text-emerald">.{{ outputExt }}</span>
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
  padding: 20px 22px;
  overflow-y: auto;
  max-height: 70vh;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.export-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.section-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.export-scope-cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 10px;
}

.scope-card {
  background-color: var(--surface-mid);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-md);
  padding: 12px 14px;
  cursor: pointer;
  transition: all 0.15s ease;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.scope-card:hover {
  background-color: var(--surface-high);
  border-color: var(--border-strong);
}

.scope-card.active {
  border-color: var(--accent-cyan);
  background-color: rgba(6, 182, 212, 0.08);
  box-shadow: 0 0 16px rgba(6, 182, 212, 0.15);
}

.scope-header {
  display: flex;
  align-items: center;
  gap: 8px;
}

.scope-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-pure);
}

.scope-desc {
  font-size: 11px;
  color: var(--text-secondary);
  line-height: 1.4;
  margin: 0;
}

.scope-meta {
  font-size: 11px;
  color: var(--text-muted);
  margin-top: auto;
  padding-top: 4px;
}

.export-mode-cards {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.mode-card {
  background-color: var(--surface-mid);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-md);
  padding: 12px 14px;
  cursor: pointer;
  transition: all 0.15s;
}

.mode-card:hover {
  background-color: var(--surface-high);
  border-color: var(--border-strong);
}

.mode-card.active {
  border-color: var(--accent-cyan);
  background-color: rgba(6, 182, 212, 0.08);
  box-shadow: 0 0 16px rgba(6, 182, 212, 0.15);
}

.mode-card-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}

.mode-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-pure);
}

.mode-badge-rec {
  font-family: var(--font-mono);
  font-size: 9px;
  background-color: var(--accent-cyan-dim);
  color: var(--accent-cyan-bright);
  padding: 1px 6px;
  border-radius: 999px;
  font-weight: 600;
}

.mode-desc {
  font-size: 11px;
  color: var(--text-secondary);
  line-height: 1.4;
}

.encoder-cards-list {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(170px, 1fr));
  gap: 8px;
}

.encoder-card {
  background-color: var(--surface-mid);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-md);
  padding: 10px 12px;
  cursor: pointer;
  transition: all 0.15s;
}

.encoder-card:hover {
  background-color: var(--surface-high);
}

.encoder-card.active {
  border-color: var(--accent-cyan);
  background-color: rgba(6, 182, 212, 0.08);
}

.encoder-header {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 4px;
}

.encoder-name {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-pure);
}

.encoder-desc {
  font-size: 10px;
  color: var(--text-muted);
}

/* Container format & Audio-only */
.section-label-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.audio-only-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  font-size: 11px;
  color: var(--accent-amber);
  user-select: none;
}

.audio-only-toggle input[type="checkbox"] {
  accent-color: var(--accent-amber);
  cursor: pointer;
}

.format-pills, .preset-pills {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.format-pill-btn, .preset-pill-btn {
  background-color: var(--surface-dim);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm);
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 600;
  padding: 6px 14px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.format-pill-btn:hover, .preset-pill-btn:hover {
  background-color: var(--surface-high);
  color: var(--text-pure);
  border-color: var(--border-strong);
}

.format-pill-btn.active {
  background-color: rgba(6, 182, 212, 0.15);
  border-color: var(--accent-cyan);
  color: var(--accent-cyan-bright);
}

.format-pill-btn.audio-pill.active {
  background-color: rgba(251, 191, 36, 0.15);
  border-color: var(--accent-amber);
  color: var(--accent-amber);
}

.preset-pill-btn.active {
  background-color: rgba(99, 102, 241, 0.2);
  border-color: var(--accent-indigo);
  color: #c7d2fe;
}

/* CRF Quality Slider */
.crf-header-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.crf-desc-badge {
  font-size: 10px;
  font-family: var(--font-mono);
  background-color: rgba(6, 182, 212, 0.12);
  color: var(--accent-cyan-bright);
  padding: 2px 8px;
  border-radius: 4px;
}

.slider-with-labels {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-bottom: 12px;
}

.crf-slider {
  width: 100%;
  accent-color: var(--accent-cyan);
  cursor: pointer;
}

.slider-ticks-row {
  display: flex;
  justify-content: space-between;
  font-size: 9px;
  font-family: var(--font-mono);
  color: var(--text-muted);
}

.sub-section-label {
  display: block;
  font-size: 11px;
  font-weight: 500;
  color: var(--text-muted);
  margin-bottom: 6px;
}

/* File picker */
.file-path-picker {
  display: flex;
  gap: 8px;
}

.input-file-path {
  flex: 1;
  background-color: var(--surface-dim);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm);
  padding: 8px 12px;
  color: var(--text-pure);
  font-family: var(--font-mono);
  font-size: 11px;
  outline: none;
}

.btn-browse-file {
  display: flex;
  align-items: center;
  gap: 6px;
  background-color: var(--surface-high);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  color: var(--text-pure);
  padding: 0 14px;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s;
}

.btn-browse-file:hover {
  background-color: var(--border-strong);
}

/* Summary */
.export-summary-box {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 10px;
  background-color: var(--surface-dim);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-md);
  padding: 12px 14px;
}

.summary-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.summary-label {
  font-size: 10px;
  color: var(--text-muted);
  text-transform: uppercase;
}

.summary-val {
  font-family: var(--font-mono);
  font-size: 12px;
  font-weight: 600;
  color: var(--text-pure);
}

.export-error-msg {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--accent-red);
  font-size: 12px;
  background-color: rgba(239, 68, 68, 0.1);
  padding: 8px 12px;
  border-radius: var(--radius-sm);
}

/* Smart Cut Plan Card */
.smartcut-plan-box {
  margin-top: 10px;
  background-color: rgba(6, 182, 212, 0.05);
  border: 1px solid rgba(6, 182, 212, 0.25);
  border-radius: var(--radius-md);
  padding: 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.smartcut-plan-header {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-pure);
}

.plan-stats-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}

.plan-stat {
  display: flex;
  flex-direction: column;
  gap: 2px;
  background-color: var(--surface-dim);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm);
  padding: 8px 10px;
}

.stat-num {
  font-family: var(--font-mono);
  font-size: 15px;
  font-weight: 700;
}

.stat-label {
  font-size: 10px;
  color: var(--text-muted);
}

.smartcut-guarantee {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--text-secondary);
  line-height: 1.35;
  border-top: 1px solid var(--border-subtle);
  padding-top: 8px;
}

.smartcut-guarantee .material-symbols-outlined {
  font-size: 16px;
  flex-shrink: 0;
}
</style>
