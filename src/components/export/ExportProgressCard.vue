<script setup lang="ts">
defineProps<{
  isExporting: boolean;
  exportFinishedSuccess: boolean;
  exportProgressPercent: number;
  exportStageText: string;
  exportFps: number;
  exportSpeed: string;
  exportMode: 'smart_cut' | 'reencode' | 'individual_clips';
  exportOutputPath: string;
}>();

const emit = defineEmits<{
  (e: 'open-file'): void;
  (e: 'open-folder'): void;
}>();
</script>

<template>
  <!-- Estado durante la exportación con progreso -->
  <div v-if="isExporting" class="export-modal-body export-progress-body">
    <div class="progress-circle-wrap">
      <div class="progress-big-number">{{ exportProgressPercent }}%</div>
      <div class="progress-spinner-ring"></div>
    </div>

    <h4 class="progress-stage-title">{{ exportStageText || 'Procesando exportación...' }}</h4>

    <div class="export-progress-bar-track">
      <div class="export-progress-bar-fill" :style="{ width: `${exportProgressPercent}%` }">
        <div class="progress-shimmer"></div>
      </div>
    </div>

    <div class="export-metrics-row">
      <span v-if="exportSpeed" class="metric-chip">Velocidad: <b>{{ exportSpeed }}</b></span>
      <span v-if="exportFps > 0" class="metric-chip">FPS: <b>{{ exportFps.toFixed(0) }}</b></span>
      <span class="metric-chip">Modo: <b>{{ exportMode === 'smart_cut' ? 'Smart Cut' : 'Recodificación' }}</b></span>
    </div>

    <p class="progress-hint">Por favor, no cierres la aplicación mientras se completa el renderizado.</p>
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
  padding: 20px 22px;
  overflow-y: auto;
  max-height: 70vh;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

/* Progress */
.export-progress-body {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 40px 20px;
}

.progress-circle-wrap {
  position: relative;
  width: 90px;
  height: 90px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 20px;
}

.progress-big-number {
  font-family: var(--font-mono);
  font-size: 22px;
  font-weight: 700;
  color: var(--accent-cyan-bright);
}

.progress-spinner-ring {
  position: absolute;
  inset: 0;
  border: 3px solid rgba(6, 182, 212, 0.15);
  border-top-color: var(--accent-cyan);
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.progress-stage-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-pure);
  margin-bottom: 16px;
}

.export-progress-bar-track {
  width: 100%;
  height: 8px;
  background-color: var(--surface-dim);
  border-radius: 999px;
  overflow: hidden;
  margin-bottom: 16px;
}

.export-progress-bar-fill {
  height: 100%;
  background: linear-gradient(90deg, #06b6d4, #6366f1);
  border-radius: 999px;
  transition: width 0.2s;
  position: relative;
  overflow: hidden;
}

.progress-shimmer {
  position: absolute;
  inset: 0;
  background: linear-gradient(
    90deg,
    rgba(255, 255, 255, 0) 0%,
    rgba(255, 255, 255, 0.4) 50%,
    rgba(255, 255, 255, 0) 100%
  );
  animation: shimmer 1.5s infinite;
}

@keyframes shimmer {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(100%); }
}

.export-metrics-row {
  display: flex;
  gap: 10px;
  margin-bottom: 16px;
}

.metric-chip {
  font-family: var(--font-mono);
  font-size: 11px;
  background-color: var(--surface-dim);
  border: 1px solid var(--border-subtle);
  padding: 3px 8px;
  border-radius: 4px;
  color: var(--text-secondary);
}

.metric-chip b {
  color: var(--text-pure);
}

.progress-hint {
  font-size: 11px;
  color: var(--text-muted);
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
