<script setup lang="ts">
import type { VideoMetadata } from '../../types';
import { formatBytes } from '../../utils/formatters';

const props = defineProps<{
  currentFilePath: string;
  metadata: VideoMetadata;
  isOpeningFile: boolean;
  errorMessage: string;
  isCodecIncompatible: boolean;
  isGeneratingPreview: boolean;
  canExport: boolean;
}>();

const emit = defineEmits<{
  (e: 'open-file'): void;
  (e: 'export'): void;
  (e: 'open-about'): void;
  (e: 'generate-preview'): void;
  (e: 'dismiss-error'): void;
}>();
</script>

<template>
  <header class="header-strip">
    <div class="header-left">
      <!-- Icono de marca minimalista -->
      <div class="brand-square-btn" title="SmartCut">
        <span class="material-symbols-outlined">content_cut</span>
      </div>

      <!-- Información del archivo cargado -->
      <div v-if="currentFilePath" class="file-info-pill">
        <span class="file-name" :title="metadata.file_path">{{ metadata.file_name }}</span>
        <span class="file-tag-res">{{ metadata.width }}x{{ metadata.height }}</span>
        <span class="file-tag-fps">{{ metadata.fps ? metadata.fps.toFixed(0) : '30' }} FPS</span>
        <span v-if="metadata.video_codec" class="file-tag-codec">{{ metadata.video_codec.toUpperCase() }}</span>
        <span v-if="metadata.file_size" class="file-tag-size">{{ formatBytes(metadata.file_size) }}</span>
      </div>
      <div v-else class="file-placeholder">
        <span class="placeholder-dot"></span>
        <span>Ningún video cargado</span>
      </div>
    </div>

    <!-- Acciones del header derecho -->
    <div class="header-actions">
      <!-- Abrir video -->
      <button
        type="button"
        class="btn-action btn-open"
        :disabled="isOpeningFile"
        @click="emit('open-file')"
        title="Abrir archivo de video [⌘O]"
      >
        <span class="material-symbols-outlined text-muted">folder_open</span>
        <span>Abrir...</span>
        <span class="shortcut-tag">⌘O</span>
      </button>

      <!-- Botón Exportar Blanco Destacado (Diseño de Referencia) -->
      <button
        type="button"
        class="btn-export-white"
        :disabled="!canExport"
        @click="emit('export')"
        title="Exportar video [⌘E]"
      >
        <span class="material-symbols-outlined">upload</span>
        <span>Exportar Video</span>
        <span class="shortcut-tag-dark">⌘E</span>
      </button>

      <!-- Botón Acerca de SmartCut -->
      <button
        type="button"
        class="btn-action btn-settings"
        @click="emit('open-about')"
        title="Acerca de SmartCut Studio"
      >
        <span class="material-symbols-outlined">info</span>
      </button>
    </div>
  </header>

  <!-- BANNER DE ERROR Y SUGERENCIA DE PROXY COMPATIBLE -->
  <transition name="slide-fade">
    <div v-if="errorMessage" class="error-banner">
      <div class="error-banner-content">
        <span class="material-symbols-outlined text-danger">error</span>
        <span>{{ errorMessage }}</span>
      </div>

      <div class="error-banner-actions">
        <button
          v-if="isCodecIncompatible && !isGeneratingPreview"
          type="button"
          class="btn-proxy-action"
          @click="emit('generate-preview')"
          title="Generar vista previa compatible ligera"
        >
          <span class="material-symbols-outlined">bolt</span>
          <span>Generar vista previa compatible</span>
        </button>
        <span v-else-if="isGeneratingPreview" class="proxy-loading-pill">
          <span class="spinner-small"></span>
          <span>Generando vista previa...</span>
        </span>

        <button type="button" class="btn-close-error" @click="emit('dismiss-error')">✕</button>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.header-strip {
  height: 48px;
  min-height: 48px;
  background-color: rgba(13, 17, 23, 0.95);
  backdrop-filter: blur(14px);
  -webkit-backdrop-filter: blur(14px);
  border-bottom: 1px solid var(--border-subtle);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 14px;
  z-index: 40;
  user-select: none;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

/* Icono de tijera minimalista */
.brand-square-btn {
  width: 32px;
  height: 32px;
  border-radius: var(--radius-sm);
  background-color: var(--surface-low);
  border: 1px solid var(--border-subtle);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-secondary);
  flex-shrink: 0;
  transition: all 0.15s;
}

.brand-square-btn:hover {
  border-color: var(--border-strong);
  color: var(--text-pure);
}

.brand-square-btn .material-symbols-outlined {
  font-size: 18px;
}

/* Píldora de metadatos */
.file-info-pill {
  display: flex;
  align-items: center;
  gap: 8px;
  background-color: rgba(18, 24, 34, 0.8);
  border: 1px solid var(--border-subtle);
  padding: 4px 10px;
  border-radius: var(--radius-sm);
  font-family: var(--font-mono);
  font-size: 11px;
}

.file-name {
  color: var(--text-primary);
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 260px;
}

.file-tag-res {
  color: #ffffff;
  font-weight: 700;
  font-size: 11px;
}

.file-tag-fps {
  color: #10b981;
  font-weight: 600;
  font-size: 11px;
}

.file-tag-codec {
  background-color: rgba(255, 255, 255, 0.08);
  color: var(--text-secondary);
  padding: 1px 5px;
  border-radius: 3px;
  font-size: 10px;
  font-weight: 600;
}

.file-tag-size {
  background-color: rgba(255, 255, 255, 0.05);
  color: var(--text-muted);
  padding: 1px 5px;
  border-radius: 3px;
  font-size: 10px;
}

.file-placeholder {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--text-muted);
  font-size: 12px;
}

.placeholder-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background-color: var(--text-muted);
}

/* Acciones */
.header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.btn-action {
  display: flex;
  align-items: center;
  gap: 6px;
  height: 32px;
  padding: 0 10px;
  border-radius: var(--radius-sm);
  font-family: var(--font-ui);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
  background-color: var(--surface-low);
  border: 1px solid var(--border-subtle);
  color: var(--text-primary);
}

.btn-action:hover:not(:disabled) {
  background-color: var(--surface-high);
  border-color: var(--border-strong);
  color: var(--text-pure);
}

.btn-action:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.btn-action .text-muted {
  color: var(--text-muted);
  font-size: 16px;
}

.shortcut-tag {
  font-family: var(--font-mono);
  font-size: 10px;
  padding: 1px 4px;
  background-color: rgba(255, 255, 255, 0.06);
  border-radius: 3px;
  color: var(--text-muted);
}

/* Botón Exportar Blanco Destacado */
.btn-export-white {
  display: flex;
  align-items: center;
  gap: 6px;
  height: 32px;
  padding: 0 12px;
  border-radius: var(--radius-sm);
  font-family: var(--font-ui);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
  background-color: #ffffff;
  border: 1px solid #ffffff;
  color: #090d16;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
}

.btn-export-white .material-symbols-outlined {
  font-size: 16px;
  color: #090d16;
}

.btn-export-white:hover:not(:disabled) {
  background-color: #f1f5f9;
  border-color: #f1f5f9;
  box-shadow: 0 2px 8px rgba(255, 255, 255, 0.25);
}

.btn-export-white:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.shortcut-tag-dark {
  font-family: var(--font-mono);
  font-size: 10px;
  padding: 1px 4px;
  background-color: rgba(0, 0, 0, 0.08);
  border-radius: 3px;
  color: #1e293b;
  font-weight: 700;
}

/* Settings */
.settings-dropdown-wrapper {
  position: relative;
}

.btn-settings {
  width: 32px;
  padding: 0;
  justify-content: center;
}

.btn-settings .material-symbols-outlined {
  font-size: 18px;
}

/* Error banner */
.error-banner {
  background-color: rgba(220, 38, 38, 0.15);
  border-bottom: 1px solid rgba(239, 68, 68, 0.3);
  padding: 8px 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  z-index: 35;
}

.error-banner-content {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: #fca5a5;
}

.error-banner-content .text-danger {
  color: #ef4444;
  font-size: 18px;
}

.error-banner-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.btn-proxy-action {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 4px 10px;
  border-radius: var(--radius-sm);
  background: linear-gradient(135deg, #f59e0b, #d97706);
  border: none;
  color: #1a0f02;
  font-family: var(--font-ui);
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
}

.btn-proxy-action:hover {
  filter: brightness(1.1);
}

.proxy-loading-pill {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: #fbbf24;
}

.btn-close-error {
  background: none;
  border: none;
  color: #fca5a5;
  cursor: pointer;
  font-size: 14px;
  padding: 2px 6px;
  border-radius: 3px;
}

.btn-close-error:hover {
  background-color: rgba(239, 68, 68, 0.2);
  color: #ffffff;
}

.spinner-small {
  width: 12px;
  height: 12px;
  border: 2px solid rgba(251, 191, 36, 0.3);
  border-top-color: #fbbf24;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
