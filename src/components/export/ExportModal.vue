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
              <span class="material-symbols-outlined text-cyan">ios_share</span>
            </div>
            <div>
              <h3 class="modal-title">Exportar Proyecto</h3>
              <p class="modal-subtitle">Genera el archivo final renderizando tus cortes con tecnología SmartCut</p>
            </div>
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
              <span class="material-symbols-outlined">rocket_launch</span>
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
  max-width: 620px;
  background-color: var(--surface-low);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-xl);
  overflow: hidden;
  box-shadow: 0 24px 60px rgba(0, 0, 0, 0.7);
  display: flex;
  flex-direction: column;
}

.export-modal-header {
  padding: 18px 22px;
  background-color: rgba(24, 28, 36, 0.85);
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
  width: 38px;
  height: 38px;
  border-radius: var(--radius-md);
  background: linear-gradient(135deg, rgba(6, 182, 212, 0.2), rgba(99, 102, 241, 0.2));
  border: 1px solid rgba(6, 182, 212, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
}

.modal-badge-icon .material-symbols-outlined {
  font-size: 22px;
}

.modal-title {
  font-family: var(--font-ui);
  font-size: 16px;
  font-weight: 600;
  color: var(--text-pure);
  letter-spacing: -0.015em;
}

.modal-subtitle {
  font-size: 12px;
  color: var(--text-secondary);
}

.btn-close-modal {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: none;
  border: none;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.15s;
}

.btn-close-modal:hover:not(:disabled) {
  background-color: var(--surface-high);
  color: var(--text-pure);
}

/* Footer */
.export-modal-footer {
  padding: 16px 22px;
  background-color: rgba(24, 28, 36, 0.85);
  border-top: 1px solid var(--border-subtle);
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
}

.btn-modal-cancel {
  background: none;
  border: 1px solid var(--border-subtle);
  color: var(--text-secondary);
  padding: 7px 16px;
  border-radius: var(--radius-sm);
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s;
}

.btn-modal-cancel:hover {
  background-color: var(--surface-high);
  color: var(--text-pure);
}

.btn-modal-start {
  display: flex;
  align-items: center;
  gap: 6px;
  background: linear-gradient(135deg, #06b6d4, #0284c7);
  border: 1px solid rgba(76, 215, 246, 0.5);
  color: #04131f;
  padding: 7px 18px;
  border-radius: var(--radius-sm);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s;
  box-shadow: 0 0 14px rgba(6, 182, 212, 0.4);
}

.btn-modal-start:hover:not(:disabled) {
  filter: brightness(1.1);
}

.btn-modal-start:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-modal-abort {
  display: flex;
  align-items: center;
  gap: 6px;
  background-color: rgba(239, 68, 68, 0.2);
  border: 1px solid rgba(239, 68, 68, 0.4);
  color: #ffdad6;
  padding: 7px 16px;
  border-radius: var(--radius-sm);
  font-size: 12px;
  cursor: pointer;
}

.btn-modal-done {
  background: linear-gradient(135deg, #06b6d4, #0284c7);
  border: none;
  color: #04131f;
  padding: 7px 20px;
  border-radius: var(--radius-sm);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}
</style>
