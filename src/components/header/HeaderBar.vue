<script setup lang="ts">
import { ref } from 'vue';
import type { VideoMetadata, SampleVideoInfo } from '../../types';
import type { ThemeOption } from '../../composables/useTheme';
import { formatBytes } from '../../utils/formatters';

const props = defineProps<{
  currentFilePath: string;
  metadata: VideoMetadata;
  sampleVideos: SampleVideoInfo[];
  isOpeningFile: boolean;
  errorMessage: string;
  isCodecIncompatible: boolean;
  isGeneratingPreview: boolean;
  canExport: boolean;
  currentThemeId: string;
  themes: ThemeOption[];
}>();

const emit = defineEmits<{
  (e: 'open-file'): void;
  (e: 'export'): void;
  (e: 'load-sample', path: string): void;
  (e: 'open-about'): void;
  (e: 'generate-preview'): void;
  (e: 'dismiss-error'): void;
  (e: 'select-theme', themeId: string): void;
  (e: 'save-project'): void;
  (e: 'open-project'): void;
  (e: 'export-edl'): void;
}>();

const isSettingsOpen = ref(false);
</script>

<template>
  <header class="header-strip">
    <div class="header-left">
      <!-- Logo de la marca SmartCut -->
      <div class="app-brand">
        <div class="brand-icon">
          <span class="material-symbols-outlined">movie_edit</span>
        </div>
        <span class="brand-title">SmartCut Studio</span>
      </div>

      <div class="divider-v"></div>

      <!-- Información del archivo cargado -->
      <div v-if="currentFilePath" class="file-info-pill">
        <span class="file-name" :title="metadata.file_path">{{ metadata.file_name }}</span>
        <span class="file-tag">{{ metadata.width }}x{{ metadata.height }}</span>
        <span class="file-tag">{{ metadata.fps ? metadata.fps.toFixed(0) : '30' }} FPS</span>
        <span v-if="metadata.video_codec" class="file-tag badge-codec">{{ metadata.video_codec.toUpperCase() }}</span>
        <span v-if="metadata.file_size" class="file-tag badge-size">{{ formatBytes(metadata.file_size) }}</span>
        <span class="direct-stream-badge" title="Lectura por demanda mediante HTTP Range 206">
          <span class="indicator-dot"></span>
          Streaming 206
        </span>
      </div>
      <div v-else class="file-placeholder">
        <span class="placeholder-dot"></span>
        <span>Ningún video cargado</span>
      </div>
    </div>

    <!-- Acciones principales del header -->
    <div class="header-actions">
      <!-- Botón de apertura de archivo nativo -->
      <button
        type="button"
        class="btn-action btn-open"
        :disabled="isOpeningFile"
        @click="emit('open-file')"
        title="Abrir archivo nativo (.mp4, .mkv, .mov, .webm) [⌘O]"
      >
        <span class="material-symbols-outlined text-cyan">folder_open</span>
        <span>Abrir...</span>
        <span class="shortcut-tag">⌘O</span>
      </button>

      <!-- Guardar y Cargar Proyecto -->
      <button
        type="button"
        class="btn-action"
        :disabled="!currentFilePath"
        @click="emit('save-project')"
        title="Guardar proyecto de corte (.smartcut) [Ctrl+S]"
      >
        <span class="material-symbols-outlined text-amber">save</span>
        <span>Guardar</span>
        <span class="shortcut-tag">⌘S</span>
      </button>

      <button
        type="button"
        class="btn-action"
        @click="emit('open-project')"
        title="Cargar sesión de proyecto (.smartcut)"
      >
        <span class="material-symbols-outlined text-indigo">file_open</span>
        <span>Proyecto...</span>
      </button>

      <button
        type="button"
        class="btn-action"
        :disabled="!canExport"
        @click="emit('export-edl')"
        title="Exportar EDL estándar CMX 3600 para Premiere / DaVinci Resolve"
      >
        <span class="material-symbols-outlined text-sky">description</span>
        <span>EDL</span>
      </button>

      <div class="divider-v"></div>

      <!-- Botón de Exportación Destacado con Aceleración GPU -->
      <button
        type="button"
        class="btn-action btn-export-highlight"
        :disabled="!canExport"
        @click="emit('export')"
        title="Exportar video editado (Smart Cut / Recodificación) [⌘E]"
      >
        <span class="material-symbols-outlined">ios_share</span>
        <span>Exportar Video</span>
        <span class="shortcut-tag-accent">⌘E</span>
      </button>

      <div class="divider-v"></div>

      <!-- Botones de muestras rápidas si están disponibles -->
      <div class="quick-samples" v-if="sampleVideos.length > 0">
        <span class="quick-label">Muestras:</span>
        <button
          v-for="sample in sampleVideos"
          :key="sample.path"
          type="button"
          class="btn-sample"
          :class="{ active: currentFilePath === sample.path }"
          @click="emit('load-sample', sample.path)"
          :title="`Cargar ${sample.name} (${sample.size_formatted})`"
        >
          <span class="sample-tag">{{ sample.format }}</span>
          <span>{{ sample.format === 'MKV' ? 'MKV' : 'MP4' }}</span>
        </button>
      </div>

      <!-- Menú de Ajustes y Acerca de -->
      <div class="settings-dropdown-wrapper">
        <button
          type="button"
          class="btn-action btn-settings"
          :class="{ active: isSettingsOpen }"
          @click.stop="isSettingsOpen = !isSettingsOpen"
          title="Configuración y Acerca de"
        >
          <span class="material-symbols-outlined">settings</span>
        </button>

        <div v-if="isSettingsOpen" class="settings-menu-popover" @click.stop>
          <div class="settings-popover-header">
            <span class="material-symbols-outlined text-cyan">palette</span>
            <span>Temas de Color</span>
          </div>

          <div class="themes-selector-list">
            <button
              v-for="t in themes"
              :key="t.id"
              type="button"
              class="theme-select-item"
              :class="{ active: currentThemeId === t.id }"
              @click="emit('select-theme', t.id)"
              :title="t.description"
            >
              <div class="theme-swatch" :style="{ backgroundColor: t.previewBg, borderColor: t.previewAccent }">
                <span class="theme-swatch-dot" :style="{ backgroundColor: t.previewAccent }"></span>
              </div>
              <div class="theme-info-col">
                <span class="theme-name">{{ t.name }}</span>
                <span class="theme-type-tag">{{ t.type === 'dark' ? 'Oscuro' : 'Claro' }}</span>
              </div>
              <span v-if="currentThemeId === t.id" class="material-symbols-outlined theme-check-icon">check_circle</span>
            </button>
          </div>

          <div class="popover-divider"></div>

          <button
            type="button"
            class="settings-menu-item"
            @click="isSettingsOpen = false; emit('open-about')"
          >
            <span class="material-symbols-outlined text-cyan">info</span>
            <span>Acerca de SmartCut</span>
          </button>
        </div>
      </div>
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
          title="Transcodifica rápidamente una copia ligera H.264 para previsualizar sin alterar el archivo original"
        >
          <span class="material-symbols-outlined">bolt</span>
          <span>Generar vista previa compatible</span>
        </button>
        <span v-else-if="isGeneratingPreview" class="proxy-loading-pill">
          <span class="spinner-small"></span>
          <span>Generando vista previa con FFmpeg...</span>
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
  background-color: rgba(15, 19, 28, 0.92);
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
  gap: 12px;
  min-width: 0;
}

.app-brand {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.brand-icon {
  width: 28px;
  height: 28px;
  border-radius: var(--radius-sm);
  background: linear-gradient(135deg, rgba(6, 182, 212, 0.25), rgba(99, 102, 241, 0.25));
  border: 1px solid rgba(6, 182, 212, 0.35);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--accent-cyan);
}

.brand-icon .material-symbols-outlined {
  font-size: 18px;
}

.brand-title {
  font-family: var(--font-ui);
  font-size: 14px;
  font-weight: 600;
  letter-spacing: -0.015em;
  color: var(--text-pure);
}

.divider-v {
  width: 1px;
  height: 20px;
  background-color: var(--border-subtle);
  flex-shrink: 0;
}

/* File info badge */
.file-info-pill {
  display: flex;
  align-items: center;
  gap: 6px;
  background-color: var(--surface-low);
  border: 1px solid var(--border-subtle);
  padding: 3px 8px;
  border-radius: var(--radius-sm);
  font-family: var(--font-mono);
  font-size: 11px;
  max-width: 580px;
}

.file-name {
  color: var(--text-pure);
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 220px;
}

.file-tag {
  background-color: var(--surface-high);
  color: var(--text-secondary);
  padding: 1px 5px;
  border-radius: var(--radius-xs);
  font-size: 10px;
}

.badge-codec {
  color: var(--accent-cyan);
  background-color: var(--accent-cyan-dim);
}

.badge-size {
  color: var(--text-secondary);
}

.direct-stream-badge {
  display: flex;
  align-items: center;
  gap: 4px;
  background-color: rgba(16, 185, 129, 0.12);
  color: #34d399;
  border: 1px solid rgba(16, 185, 129, 0.25);
  padding: 1px 6px;
  border-radius: var(--radius-xs);
  font-size: 10px;
  font-weight: 500;
}

.indicator-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background-color: #34d399;
  box-shadow: 0 0 6px rgba(52, 211, 153, 0.8);
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

/* Actions */
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
  height: 30px;
  padding: 0 10px;
  border-radius: var(--radius-sm);
  font-family: var(--font-ui);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
  background-color: var(--surface-mid);
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

.shortcut-tag {
  font-family: var(--font-mono);
  font-size: 10px;
  padding: 1px 4px;
  background-color: var(--surface-high);
  border-radius: 3px;
  color: var(--text-secondary);
}

.btn-export-highlight {
  background: linear-gradient(135deg, #06b6d4, #0284c7);
  border: 1px solid rgba(76, 215, 246, 0.5);
  color: #04131f;
  font-weight: 600;
  box-shadow: 0 0 14px rgba(6, 182, 212, 0.35);
}

.btn-export-highlight:hover:not(:disabled) {
  filter: brightness(1.1);
  box-shadow: 0 0 20px rgba(6, 182, 212, 0.55);
}

.shortcut-tag-accent {
  font-family: var(--font-mono);
  font-size: 10px;
  padding: 1px 4px;
  background-color: rgba(4, 19, 31, 0.25);
  border-radius: 3px;
  color: #04131f;
  font-weight: 700;
}

/* Quick samples */
.quick-samples {
  display: flex;
  align-items: center;
  gap: 6px;
}

.quick-label {
  color: var(--text-muted);
  font-size: 11px;
}

.btn-sample {
  display: flex;
  align-items: center;
  gap: 4px;
  height: 24px;
  padding: 0 8px;
  border-radius: var(--radius-sm);
  background-color: var(--surface-low);
  border: 1px solid var(--border-subtle);
  color: var(--text-secondary);
  font-family: var(--font-mono);
  font-size: 11px;
  cursor: pointer;
  transition: all 0.15s;
}

.btn-sample:hover {
  background-color: var(--surface-high);
  color: var(--text-pure);
  border-color: var(--accent-cyan);
}

.btn-sample.active {
  border-color: var(--accent-cyan);
  background-color: var(--accent-cyan-dim);
  color: var(--accent-cyan-bright);
}

.sample-tag {
  font-size: 9px;
  font-weight: 600;
  background-color: var(--surface-highest);
  padding: 1px 3px;
  border-radius: 2px;
}

/* Settings dropdown */
.settings-dropdown-wrapper {
  position: relative;
}

.btn-settings {
  width: 30px;
  padding: 0;
  justify-content: center;
}

.settings-menu-popover {
  position: absolute;
  top: 36px;
  right: 0;
  width: 250px;
  background-color: var(--surface-low);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-md);
  padding: 8px;
  box-shadow: 0 12px 36px rgba(0, 0, 0, 0.65);
  z-index: 100;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.settings-popover-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 6px 6px 6px;
  font-family: var(--font-ui);
  font-size: 11px;
  font-weight: 600;
  color: var(--text-pure);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.settings-popover-header .material-symbols-outlined {
  font-size: 15px;
}

.themes-selector-list {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.theme-select-item {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 6px 8px;
  background-color: var(--surface-mid);
  border: 1px solid transparent;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all 0.15s;
  text-align: left;
}

.theme-select-item:hover {
  background-color: var(--surface-high);
  border-color: var(--border-subtle);
}

.theme-select-item.active {
  background-color: rgba(6, 182, 212, 0.1);
  border-color: var(--accent-cyan);
}

.theme-swatch {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  border: 2px solid;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.theme-swatch-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}

.theme-info-col {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.theme-name {
  font-family: var(--font-ui);
  font-size: 11px;
  font-weight: 500;
  color: var(--text-pure);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.theme-type-tag {
  font-family: var(--font-mono);
  font-size: 9px;
  color: var(--text-muted);
}

.theme-check-icon {
  font-size: 16px;
  color: var(--accent-cyan);
}

.popover-divider {
  height: 1px;
  background-color: var(--border-subtle);
  margin: 4px 0;
}

.settings-menu-item {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 8px;
  background: none;
  border: none;
  color: var(--text-primary);
  font-family: var(--font-ui);
  font-size: 12px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  text-align: left;
  transition: background-color 0.15s;
}

.settings-menu-item:hover {
  background-color: var(--surface-high);
  color: var(--text-pure);
}

/* Error banner */
.error-banner {
  background-color: rgba(147, 0, 10, 0.35);
  border-bottom: 1px solid rgba(255, 180, 171, 0.3);
  padding: 8px 14px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  font-size: 12px;
  color: #ffdad6;
}

.error-banner-content {
  display: flex;
  align-items: center;
  gap: 8px;
}

.text-danger {
  color: var(--accent-red);
}

.error-banner-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.btn-proxy-action {
  display: flex;
  align-items: center;
  gap: 6px;
  background: linear-gradient(135deg, #06b6d4, #6366f1);
  color: #ffffff;
  border: none;
  border-radius: var(--radius-sm);
  padding: 4px 10px;
  font-size: 11px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
}

.btn-proxy-action:hover {
  filter: brightness(1.15);
}

.proxy-loading-pill {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--accent-cyan);
}

.spinner-small {
  width: 12px;
  height: 12px;
  border: 2px solid rgba(6, 182, 212, 0.2);
  border-top-color: var(--accent-cyan);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.btn-close-error {
  background: none;
  border: none;
  color: var(--text-secondary);
  font-size: 14px;
  cursor: pointer;
  padding: 2px 6px;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.slide-fade-enter-active,
.slide-fade-leave-active {
  transition: all 0.2s ease-out;
}
.slide-fade-enter-from,
.slide-fade-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}
</style>
