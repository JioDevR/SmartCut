<script setup lang="ts">
import type { HardwareEncoderInfo, ClipItem, VideoMetadata, ExportPlanSummary } from '../../types';
import ExportSettingsForm from './ExportSettingsForm.vue';
import ExportProgressCard from './ExportProgressCard.vue';

const props = defineProps<{
  isOpen: boolean;
  isExporting: boolean;
  exportFinishedSuccess: boolean;
  exportProgressPercent: number;
  exportStageText: string;
  exportFps: number;
  exportSpeed: string;
  exportErrorMessage: string;
  exportOutputPath: string;
  exportMode: 'smart_cut' | 'reencode' | 'individual_clips';
  availableEncoders: HardwareEncoderInfo[];
  selectedEncoder: string;
  sortedClips: ClipItem[];
  selectedClip?: ClipItem | null;
  exportScope?: 'selected' | 'timeline';
  metadata: VideoMetadata;
  containerFormat?: 'mp4' | 'mkv' | 'mov' | 'webm';
  crf?: number;
  preset?: 'ultrafast' | 'fast' | 'medium' | 'slow';
  audioOnly?: boolean;
  audioFormat?: 'mp3' | 'aac' | 'flac';
  planSummary?: ExportPlanSummary | null;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'update:exportMode', mode: 'smart_cut' | 'reencode' | 'individual_clips'): void;
  (e: 'update:exportScope', scope: 'selected' | 'timeline'): void;
  (e: 'update:selectedEncoder', enc: string): void;
  (e: 'update:containerFormat', fmt: 'mp4' | 'mkv' | 'mov' | 'webm'): void;
  (e: 'update:crf', crf: number): void;
  (e: 'update:preset', preset: 'ultrafast' | 'fast' | 'medium' | 'slow'): void;
  (e: 'update:audioOnly', audioOnly: boolean): void;
  (e: 'update:audioFormat', fmt: 'mp3' | 'aac' | 'flac'): void;
  (e: 'browse-path'): void;
  (e: 'start-export'): void;
  (e: 'cancel-export'): void;
  (e: 'open-file'): void;
  (e: 'open-folder'): void;
}>();
</script>

<template>
  <Teleport to="body">
    <div v-if="isOpen" class="export-modal-backdrop" @click.self="!isExporting && emit('close')">
      <div class="export-modal-card">
        <!-- Cabecera del modal -->
        <div class="export-modal-header">
          <div class="modal-title-group">
            <div class="modal-badge-icon">
              <span class="material-symbols-outlined text-green">upload</span>
            </div>
            <h3 class="modal-title">Exportar Proyecto</h3>
          </div>
          <button
            type="button"
            class="btn-close-modal"
            :disabled="isExporting"
            @click="emit('close')"
            title="Cerrar ventana"
          >
            <span class="material-symbols-outlined">close</span>
          </button>
        </div>

        <!-- 1. Formulario de configuración inicial -->
        <ExportSettingsForm
          v-if="!isExporting && !exportFinishedSuccess"
          :export-output-path="exportOutputPath"
          :export-mode="exportMode"
          :export-scope="exportScope"
          :selected-clip="selectedClip"
          :available-encoders="availableEncoders"
          :selected-encoder="selectedEncoder"
          :sorted-clips="sortedClips"
          :metadata="metadata"
          :container-format="containerFormat"
          :crf="crf"
          :preset="preset"
          :audio-only="audioOnly"
          :audio-format="audioFormat"
          :plan-summary="planSummary"
          :export-error-message="exportErrorMessage"
          @update:export-mode="emit('update:exportMode', $event)"
          @update:export-scope="emit('update:exportScope', $event)"
          @update:selected-encoder="emit('update:selectedEncoder', $event)"
          @update:container-format="emit('update:containerFormat', $event)"
          @update:crf="emit('update:crf', $event)"
          @update:preset="emit('update:preset', $event)"
          @update:audio-only="emit('update:audioOnly', $event)"
          @update:audio-format="emit('update:audioFormat', $event)"
          @browse-path="emit('browse-path')"
        />

        <!-- 2. Tarjeta de progreso y éxito -->
        <ExportProgressCard
          v-else
          :is-exporting="isExporting"
          :export-finished-success="exportFinishedSuccess"
          :export-progress-percent="exportProgressPercent"
          :export-stage-text="exportStageText"
          :export-fps="exportFps"
          :export-speed="exportSpeed"
          :export-mode="exportMode"
          :export-output-path="exportOutputPath"
          @open-file="emit('open-file')"
          @open-folder="emit('open-folder')"
        />

        <!-- Pie del modal -->
        <div class="export-modal-footer">
          <template v-if="!isExporting && !exportFinishedSuccess">
            <button type="button" class="btn-modal-cancel" @click="emit('close')">
              Cancelar
            </button>
            <button
              type="button"
              class="btn-modal-start"
              :disabled="sortedClips.length === 0"
              @click="emit('start-export')"
            >
              <span class="material-symbols-outlined">send</span>
              <span>Iniciar Exportación</span>
            </button>
          </template>

          <template v-else-if="isExporting">
            <button type="button" class="btn-modal-abort" @click="emit('cancel-export')">
              <span class="material-symbols-outlined">stop_circle</span>
              <span>Cancelar Renderizado</span>
            </button>
          </template>

          <template v-else-if="exportFinishedSuccess">
            <button type="button" class="btn-modal-done" @click="emit('close')">
              Aceptar y Cerrar
            </button>
          </template>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.export-modal-backdrop {
  position: fixed;
  inset: 0;
  background-color: rgba(5, 8, 14, 0.82);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 16px;
}

.export-modal-card {
  width: 100%;
  max-width: 600px;
  background-color: var(--surface-dim);
  border: 1px solid var(--border-subtle);
  border-radius: 14px;
  overflow: hidden;
  box-shadow: 0 24px 60px rgba(0, 0, 0, 0.7);
  display: flex;
  flex-direction: column;
}

.export-modal-header {
  padding: 16px 24px;
  background-color: var(--surface-low);
  border-bottom: 1px solid var(--border-subtle);
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.modal-title-group {
  display: flex;
  align-items: center;
  gap: 12px;
}

.modal-badge-icon {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  background: var(--accent-primary-dim);
  border: 1px solid rgba(16, 185, 129, 0.25);
  display: flex;
  align-items: center;
  justify-content: center;
}

.modal-badge-icon .material-symbols-outlined {
  font-size: 20px;
  color: var(--accent-primary);
}

.modal-title {
  font-family: var(--font-ui);
  font-size: 16px;
  font-weight: 700;
  color: #ffffff;
  letter-spacing: -0.015em;
  margin: 0;
}

.btn-close-modal {
  width: 32px;
  height: 32px;
  border-radius: 6px;
  background: none;
  border: none;
  color: var(--text-muted);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.15s;
}

.btn-close-modal:hover:not(:disabled) {
  background-color: var(--surface-high);
  color: #ffffff;
}

/* Footer */
.export-modal-footer {
  padding: 16px 24px;
  background-color: var(--surface-low);
  border-top: 1px solid var(--border-subtle);
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 12px;
}

.btn-modal-cancel {
  background-color: var(--surface-high);
  border: 1px solid var(--border-strong);
  color: var(--text-primary);
  padding: 8px 18px;
  border-radius: 8px;
  font-size: 12.5px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
}

.btn-modal-cancel:hover {
  background-color: var(--surface-highest);
  color: #ffffff;
}

.btn-modal-start {
  display: flex;
  align-items: center;
  gap: 8px;
  background-color: #10b981;
  border: none;
  color: #000000;
  padding: 8px 20px;
  border-radius: 8px;
  font-size: 12.5px;
  font-weight: 700;
  cursor: pointer;
  transition: all 0.15s;
  box-shadow: 0 0 16px rgba(16, 185, 129, 0.25);
}

.btn-modal-start .material-symbols-outlined {
  font-size: 16px;
  color: #000000;
  transform: rotate(-15deg);
}

.btn-modal-start:hover:not(:disabled) {
  background-color: #34d399;
  box-shadow: 0 0 20px rgba(52, 211, 153, 0.4);
}

.btn-modal-start:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  box-shadow: none;
}

.btn-modal-abort {
  display: flex;
  align-items: center;
  gap: 6px;
  background-color: rgba(239, 68, 68, 0.2);
  border: 1px solid rgba(239, 68, 68, 0.4);
  color: #ffdad6;
  padding: 8px 18px;
  border-radius: 8px;
  font-size: 12.5px;
  cursor: pointer;
}

.btn-modal-done {
  background-color: #10b981;
  border: none;
  color: #000000;
  padding: 8px 22px;
  border-radius: 8px;
  font-size: 12.5px;
  font-weight: 700;
  cursor: pointer;
}
</style>
