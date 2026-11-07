<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  isExporting: boolean;
  exportFinishedSuccess: boolean;
  exportProgressPercent: number;
  exportStageText: string;
  exportFps?: number;
  exportSpeed?: string;
  exportMode?: 'smart_cut' | 'reencode' | 'individual_clips';
  exportOutputPath: string;
}>();

const emit = defineEmits<{
  (e: 'open-file'): void;
  (e: 'open-folder'): void;
}>();

const radius = 54;
const circumference = 2 * Math.PI * radius; // ~339.292

const strokeDashoffset = computed(() => {
  const p = Math.min(100, Math.max(0, props.exportProgressPercent));
  return circumference - (p / 100) * circumference;
});
</script>

<template>
  <!-- Estado durante la exportación con solo círculo de progreso -->
  <div v-if="isExporting" class="export-modal-body export-progress-body">
    <div class="progress-circle-wrap">
      <svg class="progress-svg" viewBox="0 0 128 128">
        <!-- Círculo de pista de fondo -->
        <circle
          class="progress-circle-bg"
          cx="64"
          cy="64"
          :r="radius"
        />
        <!-- Círculo de avance en tiempo real -->
        <circle
          class="progress-circle-fill"
          cx="64"
          cy="64"
          :r="radius"
          :style="{
            strokeDasharray: `${circumference}`,
            strokeDashoffset: `${strokeDashoffset}`
          }"
        />
      </svg>
      <div class="progress-percentage-display">
        <span class="progress-big-number">{{ exportProgressPercent }}%</span>
      </div>
    </div>

    <!-- Información de fase y telemetría de exportación -->
    <div class="progress-info-block">
      <p class="progress-stage-text">{{ exportStageText || 'Procesando exportación...' }}</p>
      <div v-if="(exportFps && exportFps > 0) || exportSpeed" class="progress-telemetry-row">
        <span v-if="exportFps && exportFps > 0" class="telemetry-pill">
          <span class="material-symbols-outlined tele-icon">speed</span>
          {{ exportFps.toFixed(0) }} FPS
        </span>
        <span v-if="exportSpeed" class="telemetry-pill">
          <span class="material-symbols-outlined tele-icon">bolt</span>
          {{ exportSpeed }}
        </span>
      </div>
    </div>
  </div>

  <!-- Estado al finalizar la exportación con éxito -->
  <div v-else-if="exportFinishedSuccess" class="export-modal-body export-success-body">
    <div class="success-icon-ring">
      <span class="material-symbols-outlined">check_circle</span>
    </div>
    <h4 class="success-title">¡Exportación Completada!</h4>
    <p class="success-subtitle">Tu video se ha generado y guardado correctamente en tu disco.</p>

    <div class="success-path-pill" :title="exportOutputPath">
      <span class="material-symbols-outlined">description</span>
      <span class="path-text">{{ exportOutputPath }}</span>
    </div>

    <div class="success-actions-row">
      <button type="button" class="btn-success-action" @click="emit('open-file')">
        <span class="material-symbols-outlined">play_circle</span>
        <span>Reproducir Video</span>
      </button>
      <button type="button" class="btn-success-action" @click="emit('open-folder')">
        <span class="material-symbols-outlined">folder</span>
        <span>Ver en Carpeta</span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.export-modal-body {
  padding: 24px;
  overflow-y: auto;
  max-height: 70vh;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

/* Progress Minimalista */
.export-progress-body {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 20px;
}

.progress-circle-wrap {
  position: relative;
  width: 140px;
  height: 140px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.progress-svg {
  width: 100%;
  height: 100%;
  transform: rotate(-90deg);
}

.progress-circle-bg {
  fill: none;
  stroke: rgba(255, 255, 255, 0.08);
  stroke-width: 6;
}

.progress-circle-fill {
  fill: none;
  stroke: var(--accent-primary);
  stroke-width: 6;
  stroke-linecap: round;
  transition: stroke-dashoffset 0.25s ease-out;
  filter: drop-shadow(0 0 8px rgba(16, 185, 129, 0.45));
}

.progress-percentage-display {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.progress-big-number {
  font-family: var(--font-mono);
  font-size: 28px;
  font-weight: 700;
  color: var(--accent-primary-bright);
  letter-spacing: -0.02em;
}

.progress-info-block {
  margin-top: 18px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  text-align: center;
  max-width: 380px;
}

.progress-stage-text {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
  line-height: 1.4;
}

.progress-telemetry-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.telemetry-pill {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 10px;
  background-color: var(--surface-dim);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-full);
  font-size: 11px;
  font-family: var(--font-mono);
  color: var(--text-secondary);
}

.tele-icon {
  font-size: 14px;
  color: var(--accent-primary);
}

/* Success */
.export-success-body {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 36px 20px;
}

.success-icon-ring {
  width: 64px;
  height: 64px;
  border-radius: 50%;
  background-color: rgba(16, 185, 129, 0.15);
  border: 1px solid rgba(16, 185, 129, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  color: #34d399;
  margin-bottom: 16px;
}

.success-icon-ring .material-symbols-outlined {
  font-size: 36px;
}

.success-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-pure);
  margin-bottom: 6px;
}

.success-subtitle {
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 20px;
}

.success-path-pill {
  display: flex;
  align-items: center;
  gap: 8px;
  background-color: var(--surface-dim);
  border: 1px solid var(--border-subtle);
  padding: 6px 12px;
  border-radius: var(--radius-sm);
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--text-secondary);
  max-width: 480px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-bottom: 22px;
}

.success-actions-row {
  display: flex;
  gap: 12px;
}

.btn-success-action {
  display: flex;
  align-items: center;
  gap: 6px;
  background-color: var(--surface-high);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  color: var(--text-pure);
  padding: 8px 16px;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
}

.btn-success-action:hover {
  background-color: var(--border-strong);
}
</style>
