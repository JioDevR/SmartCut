<script setup lang="ts">
import { computed, inject, type Ref } from 'vue';
import type { ActiveTool, VideoMetadata } from '../../types';
import { formatTimecode } from '../../utils/formatters';

const props = defineProps<{
  activeTool: ActiveTool;
  canUndo?: boolean;
  canRedo?: boolean;
  isPlaying: boolean;
  pixelsPerSecond: number;
  minPixelsPerSecond: number;
  maxPixelsPerSecond: number;
  currentTime?: number;
  duration: number;
  metadata: VideoMetadata;
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
  (e: 'toggle-play'): void;
  (e: 'step-frame', delta: number): void;
  (e: 'step-seconds', delta: number): void;
  (e: 'zoom-in'): void;
  (e: 'zoom-out'): void;
  (e: 'set-zoom', pps: number): void;
}>();
</script>

<template>
  <div class="timeline-editor-toolbar">
    <!-- GRUPO IZQUIERDO: Herramientas de edición y Deshacer/Rehacer -->
    <div class="tb-group tb-left-tools">
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
          title="Herramienta Cuchilla [C]"
        >
          <span class="material-symbols-outlined">content_cut</span>
          <span>[C] Cuchilla</span>
        </button>
      </div>

      <button
        type="button"
        class="tb-btn tb-btn-split"
        @click="emit('split-at-playhead')"
        title="Dividir en la posición del cabezal [S]"
      >
        <span class="material-symbols-outlined">call_split</span>
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

    <!-- GRUPO CENTRAL: Transporte y Reproducción Integrado -->
    <div class="tb-group tb-center-transport">
      <!-- Retroceder 10s -->
      <button
        type="button"
        class="transport-icon-btn"
        @click="emit('step-seconds', -10)"
        title="Retroceder 10s"
      >
        <span class="material-symbols-outlined">replay_10</span>
      </button>

      <!-- Cuadro / corte anterior -->
      <button
        type="button"
        class="transport-icon-btn"
        @click="emit('step-frame', -1)"
        title="Cuadro anterior"
      >
        <span class="material-symbols-outlined">skip_previous</span>
      </button>

      <!-- Play / Pause Principal Redondo Destacado -->
      <button
        type="button"
        class="transport-master-play"
        @click="emit('toggle-play')"
        :title="isPlaying ? 'Pausar [Espacio]' : 'Reproducir [Espacio]'"
      >
        <span class="material-symbols-outlined">
          {{ isPlaying ? 'pause' : 'play_arrow' }}
        </span>
      </button>

      <!-- Cuadro / corte siguiente -->
      <button
        type="button"
        class="transport-icon-btn"
        @click="emit('step-frame', 1)"
        title="Cuadro siguiente"
      >
        <span class="material-symbols-outlined">skip_next</span>
      </button>

      <!-- Avanzar 10s -->
      <button
        type="button"
        class="transport-icon-btn"
        @click="emit('step-seconds', 10)"
        title="Avanzar 10s"
      >
        <span class="material-symbols-outlined">forward_10</span>
      </button>
    </div>

    <!-- GRUPO DERECHO: Zoom y Timecode minimalista -->
    <div class="tb-group tb-right-utils">
      <!-- Zoom Slider -->
      <div class="zoom-cluster">
        <button
          type="button"
          class="zoom-symbol-btn"
          @click="emit('zoom-out')"
          title="Alejar Zoom"
        >
          —
        </button>

        <input
          type="range"
          :min="minPixelsPerSecond"
          :max="maxPixelsPerSecond"
          step="2"
          class="zoom-slider-green"
          :value="pixelsPerSecond"
          @input="emit('set-zoom', Number(($event.target as HTMLInputElement).value))"
          title="Zoom del timeline"
        />

        <button
          type="button"
          class="zoom-symbol-btn"
          @click="emit('zoom-in')"
          title="Acercar Zoom"
        >
          +
        </button>
      </div>

      <!-- Timecode: Cabezal / Duración Total -->
      <div class="clean-timecode-box">
        <span class="tc-current">{{ formatTimecode(displayCurrentTime, metadata.fps) }}</span>
        <span class="tc-sep">/</span>
        <span class="tc-total">{{ formatTimecode(duration, metadata.fps) }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.timeline-editor-toolbar {
  height: 44px;
  background-color: rgba(14, 18, 26, 0.95);
  backdrop-filter: blur(14px);
  border-bottom: 1px solid var(--border-subtle);
  border-top: 1px solid var(--border-subtle);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 14px;
  position: relative;
  user-select: none;
}

.tb-group {
  display: flex;
  align-items: center;
  gap: 8px;
}

/* Herramientas Izquierda */
.tb-segmented-tools {
  display: flex;
  align-items: center;
  background-color: rgba(22, 29, 41, 0.85);
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
  background-color: rgba(255, 255, 255, 0.08);
  color: #10b981;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
}

.tb-btn {
  display: flex;
  align-items: center;
  gap: 5px;
  height: 28px;
  padding: 0 10px;
  border-radius: var(--radius-xs);
  background-color: var(--surface-low);
  border: 1px solid var(--border-subtle);
  color: var(--text-secondary);
  font-family: var(--font-ui);
  font-size: 11px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
}

.tb-btn:hover {
  background-color: var(--surface-high);
  color: var(--text-pure);
  border-color: var(--border-strong);
}

.tb-btn-split .material-symbols-outlined {
  font-size: 14px;
  color: var(--text-secondary);
}

.tb-icon-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: var(--radius-xs);
  background-color: transparent;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.15s;
}

.tb-icon-btn:hover:not(:disabled) {
  background-color: var(--surface-high);
  color: var(--text-pure);
}

.tb-icon-btn:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}

.tb-icon-btn .material-symbols-outlined {
  font-size: 16px;
}

/* Transporte Central */
.tb-center-transport {
  position: absolute;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 8px;
}

.transport-icon-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border-radius: 50%;
  background: transparent;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.15s;
}

.transport-icon-btn:hover {
  background-color: rgba(255, 255, 255, 0.08);
  color: var(--text-pure);
}

.transport-icon-btn .material-symbols-outlined {
  font-size: 18px;
}

.transport-master-play {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  background-color: #ffffff;
  border: 1px solid #ffffff;
  color: #090d16;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.15s ease;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
}

.transport-master-play:hover {
  transform: scale(1.06);
  background-color: #f1f5f9;
  box-shadow: 0 0 14px rgba(255, 255, 255, 0.3);
}

.transport-master-play .material-symbols-outlined {
  font-size: 22px;
}

/* Zoom y Timecode Derecho */
.tb-right-utils {
  display: flex;
  align-items: center;
  gap: 16px;
}

.zoom-cluster {
  display: flex;
  align-items: center;
  gap: 6px;
}

.zoom-symbol-btn {
  background: none;
  border: none;
  color: var(--text-secondary);
  font-size: 13px;
  cursor: pointer;
  padding: 0 4px;
}

.zoom-symbol-btn:hover {
  color: var(--text-pure);
}

.zoom-slider-green {
  width: 90px;
  height: 4px;
  -webkit-appearance: none;
  appearance: none;
  background: rgba(255, 255, 255, 0.15);
  border-radius: 2px;
  outline: none;
}

.zoom-slider-green::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: #10b981;
  cursor: pointer;
  box-shadow: 0 0 6px rgba(16, 185, 129, 0.6);
  transition: transform 0.1s;
}

.zoom-slider-green::-webkit-slider-thumb:hover {
  transform: scale(1.2);
}

.clean-timecode-box {
  display: flex;
  align-items: center;
  gap: 6px;
  font-family: var(--font-mono);
  font-size: 12px;
}

.tc-current {
  color: #10b981;
  font-weight: 600;
  letter-spacing: -0.02em;
}

.tc-sep {
  color: var(--text-muted);
}

.tc-total {
  color: var(--text-secondary);
  letter-spacing: -0.02em;
}
</style>
