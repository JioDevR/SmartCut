<script setup lang="ts">
import { computed, inject, type Ref } from 'vue';
import type { ActiveTool, AudioVisMode, VideoMetadata } from '../../types';
import { formatTimecode } from '../../utils/formatters';

const props = defineProps<{
  activeTool: ActiveTool;
  canUndo?: boolean;
  canRedo?: boolean;
  pixelsPerSecond: number;
  minPixelsPerSecond: number;
  maxPixelsPerSecond: number;
  audioVisMode: AudioVisMode;
  playbackRate: number;
  currentTime?: number;
  duration: number;
  metadata: VideoMetadata;
  isSidebarCollapsed: boolean;
}>();

const playheadState = inject<{ currentTime: Ref<number> } | null>('timelinePlayheadState', null);
const displayCurrentTime = computed(() => {
  return playheadState ? playheadState.currentTime.value : (props.currentTime ?? 0);
});

const emit = defineEmits<{
  (e: 'set-tool', tool: ActiveTool): void;
  (e: 'split-at-playhead'): void;
  (e: 'undo'): void;
  (e: 'redo'): void;
  (e: 'zoom-in'): void;
  (e: 'zoom-out'): void;
  (e: 'set-zoom', pps: number): void;
  (e: 'fit-to-screen'): void;
  (e: 'toggle-audio-vis-mode'): void;
  (e: 'set-playback-rate', rate: number): void;
  (e: 'toggle-sidebar'): void;
}>();

function nextPlaybackRate() {
  const rates = [0.5, 1.0, 1.5, 2.0];
  const curIdx = rates.indexOf(props.playbackRate);
  const next = rates[(curIdx + 1) % rates.length];
  emit('set-playback-rate', next);
}
</script>

<template>
  <div class="timeline-editor-toolbar">
    <!-- Modos de herramienta: Selección y Cuchilla -->
    <div class="tb-group">
      <div class="tb-segmented-tools">
        <button
          type="button"
          class="tb-tool-btn"
          :class="{ active: activeTool === 'select' }"
          @click="emit('set-tool', 'select')"
          title="Herramienta Selección [V]"
        >
          <span class="material-symbols-outlined">near_me</span>
          <span>[V] Selección</span>
        </button>

        <button
          type="button"
          class="tb-tool-btn"
          :class="{ active: activeTool === 'cut' }"
          @click="emit('set-tool', 'cut')"
          title="Herramienta Cuchilla / Corte [C]"
        >
          <span class="material-symbols-outlined text-amber">content_cut</span>
          <span>[C] Cuchilla</span>
        </button>
      </div>

      <button
        type="button"
        class="tb-btn tb-btn-split"
        @click="emit('split-at-playhead')"
        title="Cortar en posición del cabezal [S]"
      >
        <span class="material-symbols-outlined text-amber">content_cut</span>
        <span>Dividir [S]</span>
      </button>

      <button
        type="button"
        class="tb-icon-btn"
        :disabled="!canUndo"
        @click="emit('undo')"
        title="Deshacer [Ctrl+Z]"
      >
        <span class="material-symbols-outlined">undo</span>
      </button>

      <button
        type="button"
        class="tb-icon-btn"
        :disabled="!canRedo"
        @click="emit('redo')"
        title="Rehacer [Ctrl+Shift+Z / Ctrl+Y]"
      >
        <span class="material-symbols-outlined">redo</span>
      </button>
    </div>

    <!-- Zoom y ajuste central -->
    <div class="tb-group tb-zoom-group">
      <button
        type="button"
        class="tb-icon-btn"
        @click="emit('zoom-out')"
        title="Reducir Zoom (Ctrl -)"
      >
        <span class="material-symbols-outlined">remove</span>
      </button>

      <input
        type="range"
        :min="minPixelsPerSecond"
        :max="maxPixelsPerSecond"
        step="2"
        class="zoom-slider"
        :value="pixelsPerSecond"
        @input="emit('set-zoom', Number(($event.target as HTMLInputElement).value))"
        title="Nivel de zoom"
      />

      <button
        type="button"
        class="tb-icon-btn"
        @click="emit('zoom-in')"
        title="Aumentar Zoom (Ctrl +)"
      >
        <span class="material-symbols-outlined">add</span>
      </button>

      <div class="tb-divider"></div>

      <button
        type="button"
        class="tb-btn tb-fit-btn"
        @click="emit('fit-to-screen')"
        title="Ajustar a Pantalla [⌥Z]"
      >
        <span class="material-symbols-outlined">fit_screen</span>
        <span>Ajustar [⌥Z]</span>
      </button>
    </div>

    <!-- Timecode y controles auxiliares derechos -->
    <div class="tb-group tb-right-group">
      <!-- Modo de visualización de audio -->
      <button
        type="button"
        class="tb-icon-btn"
        @click="emit('toggle-audio-vis-mode')"
        :title="`Modo de Audio: ${audioVisMode === 'waves' ? 'Onda' : 'Espectrograma'}`"
      >
        <span class="material-symbols-outlined text-cyan">
          {{ audioVisMode === 'waves' ? 'graphic_eq' : 'equalizer' }}
        </span>
      </button>

      <!-- Velocidad -->
      <button
        type="button"
        class="tb-btn tb-speed-btn"
        @click="nextPlaybackRate"
        title="Velocidad de reproducción"
      >
        {{ playbackRate }}x
      </button>

      <div class="tb-divider"></div>

      <!-- Cápsula de Timecode de Cabezal -->
      <div class="timecode-display-capsule">
        <span class="tc-label">CABEZAL:</span>
        <span class="tc-active">{{ formatTimecode(displayCurrentTime, metadata.fps) }}</span>
        <span class="tc-total">/ {{ formatTimecode(duration, metadata.fps) }}</span>
      </div>

      <div class="tb-divider"></div>

      <!-- Toggle de panel lateral de clips -->
      <button
        type="button"
        class="tb-icon-btn"
        :class="{ active: !isSidebarCollapsed }"
        @click="emit('toggle-sidebar')"
        :title="isSidebarCollapsed ? 'Mostrar panel de clips' : 'Ocultar panel de clips'"
      >
        <span class="material-symbols-outlined">view_sidebar</span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.timeline-editor-toolbar {
  height: 42px;
  background-color: rgba(19, 27, 42, 0.85);
  backdrop-filter: blur(12px);
  border-bottom: 1px solid var(--border-subtle);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px;
}

.tb-group {
  display: flex;
  align-items: center;
  gap: 8px;
}

.tb-segmented-tools {
  display: flex;
  align-items: center;
  background-color: var(--surface-dim);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm);
  padding: 2px;
  gap: 2px;
}

.tb-tool-btn {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 4px 8px;
  border-radius: var(--radius-xs);
  background: none;
  border: none;
  color: var(--text-secondary);
  font-family: var(--font-ui);
  font-size: 11px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
}

.tb-tool-btn .material-symbols-outlined {
  font-size: 14px;
}

.tb-tool-btn:hover {
  color: var(--text-pure);
}

.tb-tool-btn.active {
  background-color: var(--surface-high);
  color: var(--accent-cyan);
  font-weight: 600;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.3);
}

.tb-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  border-radius: var(--radius-xs);
  background-color: var(--surface-mid);
  border: 1px solid var(--border-subtle);
  color: var(--text-primary);
  font-family: var(--font-ui);
  font-size: 11px;
  cursor: pointer;
  transition: all 0.15s;
}

.tb-btn:hover {
  background-color: var(--surface-high);
  color: var(--text-pure);
}

.tb-btn .material-symbols-outlined {
  font-size: 14px;
}

.tb-icon-btn {
  width: 26px;
  height: 26px;
  border-radius: var(--radius-xs);
  background: none;
  border: none;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.15s;
}

.tb-icon-btn .material-symbols-outlined {
  font-size: 16px;
}

.tb-icon-btn:hover {
  background-color: var(--surface-high);
  color: var(--text-pure);
}

.tb-icon-btn.active {
  color: var(--accent-cyan);
  background-color: var(--accent-cyan-dim);
}

.zoom-slider {
  width: 90px;
  height: 4px;
  appearance: none;
  background: var(--surface-highest);
  border-radius: 999px;
  outline: none;
  cursor: pointer;
}

.zoom-slider::-webkit-slider-thumb {
  appearance: none;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--accent-cyan);
  cursor: pointer;
}

.tb-divider {
  width: 1px;
  height: 16px;
  background-color: var(--border-subtle);
}

.timecode-display-capsule {
  display: flex;
  align-items: baseline;
  gap: 4px;
  background-color: var(--surface-dim);
  border: 1px solid var(--border-subtle);
  padding: 2px 8px;
  border-radius: var(--radius-sm);
  font-family: var(--font-mono);
}

.tc-label {
  font-size: 9px;
  color: var(--text-muted);
  font-weight: 600;
}

.tc-active {
  font-size: 12px;
  font-weight: 700;
  color: var(--accent-cyan-bright);
}

.tc-total {
  font-size: 10px;
  color: var(--text-muted);
}
</style>
