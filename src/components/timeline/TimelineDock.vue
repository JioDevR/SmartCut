<script setup lang="ts">
import { computed, ref } from 'vue';
import type { ClipItem, TimelineGap, TimelineTile, RulerMark, ActiveTool, AudioVisMode, VideoMetadata } from '../../types';
import { formatTimecode } from '../../utils/formatters';
import TimelineToolbar from './TimelineToolbar.vue';
import TimelineRuler from './TimelineRuler.vue';
import TimelinePlayhead from './TimelinePlayhead.vue';

const props = defineProps<{
  duration: number;
  pixelsPerSecond: number;
  minPixelsPerSecond: number;
  maxPixelsPerSecond: number;
  activeTool: ActiveTool;
  clips: ClipItem[];
  selectedClipId: string | null;
  splitPoints: number[];
  timelineGaps: TimelineGap[];
  audioWaveformUrl: string;
  audioVisMode: AudioVisMode;
  metadata: VideoMetadata;
  rulerMarks: RulerMark[];
  timelineTrackWidth: number;
  isSidebarCollapsed: boolean;
  playbackRate: number;
  getTilesForClip: (clip: ClipItem) => TimelineTile[];
  canUndo?: boolean;
  canRedo?: boolean;
}>();

const emit = defineEmits<{
  (e: 'set-tool', tool: ActiveTool): void;
  (e: 'split-at-playhead'): void;
  (e: 'cut-at-time', time: number): void;
  (e: 'select-clip', id: string): void;
  (e: 'jump-to-clip-start', clip: ClipItem): void;
  (e: 'close-gap', gap: TimelineGap): void;
  (e: 'context-menu', event: MouseEvent, clip: ClipItem): void;
  (e: 'set-zoom', pps: number): void;
  (e: 'zoom-in'): void;
  (e: 'zoom-out'): void;
  (e: 'fit-to-screen'): void;
  (e: 'toggle-audio-vis-mode'): void;
  (e: 'toggle-sidebar'): void;
  (e: 'set-playback-rate', rate: number): void;
  (e: 'track-pointer-down', event: PointerEvent): void;
  (e: 'track-pointer-move', event: PointerEvent): void;
  (e: 'track-pointer-up', event: PointerEvent): void;
  (e: 'track-pointer-leave'): void;
  (e: 'track-wheel', event: WheelEvent): void;
  (e: 'track-scroll', event: Event): void;
  (e: 'clip-pointer-down', event: PointerEvent, clip: ClipItem): void;
  (e: 'clip-pointer-move', event: PointerEvent, clip: ClipItem): void;
  (e: 'clip-pointer-up', event: PointerEvent, clip: ClipItem): void;
  (e: 'trim-handle-down', payload: { event: PointerEvent; edge: 'left' | 'right'; clip: ClipItem }): void;
  (e: 'undo'): void;
  (e: 'redo'): void;
}>();

const timelineTrack = ref<HTMLElement | null>(null);
const timelineScrollContainer = ref<HTMLElement | null>(null);

const clipTilesMap = computed(() => {
  const map: Record<string, TimelineTile[]> = {};
  for (const clip of props.clips) {
    map[clip.id] = props.getTilesForClip(clip);
  }
  return map;
});

defineExpose({
  timelineTrack,
  timelineScrollContainer,
});
</script>

<template>
  <footer class="timeline-dock">
    <!-- 1. BARRA DE HERRAMIENTAS PRO DE LA LÍNEA DE TIEMPO -->
    <TimelineToolbar
      :active-tool="activeTool"
      :can-undo="canUndo"
      :can-redo="canRedo"
      :pixels-per-second="pixelsPerSecond"
      :min-pixels-per-second="minPixelsPerSecond"
      :max-pixels-per-second="maxPixelsPerSecond"
      :audio-vis-mode="audioVisMode"
      :playback-rate="playbackRate"
      :duration="duration"
      :metadata="metadata"
      :is-sidebar-collapsed="isSidebarCollapsed"
      @set-tool="emit('set-tool', $event)"
      @split-at-playhead="emit('split-at-playhead')"
      @undo="emit('undo')"
      @redo="emit('redo')"
      @zoom-in="emit('zoom-in')"
      @zoom-out="emit('zoom-out')"
      @set-zoom="emit('set-zoom', $event)"
      @fit-to-screen="emit('fit-to-screen')"
      @toggle-audio-vis-mode="emit('toggle-audio-vis-mode')"
      @set-playback-rate="emit('set-playback-rate', $event)"
      @toggle-sidebar="emit('toggle-sidebar')"
    />

    <!-- 2. VIEWPORT SCROLLEABLE DE LA LÍNEA DE TIEMPO -->
    <div
      ref="timelineScrollContainer"
      class="timeline-scroll-viewport"
      @wheel="emit('track-wheel', $event)"
      @scroll="emit('track-scroll', $event)"
    >
      <div
        ref="timelineTrack"
        class="timeline-track-canvas"
        :class="activeTool === 'cut' ? 'tool-cut' : 'tool-select'"
        :style="{ width: `${timelineTrackWidth}px` }"
        @pointerdown="emit('track-pointer-down', $event)"
        @pointermove="emit('track-pointer-move', $event)"
        @pointerup="emit('track-pointer-up', $event)"
        @pointercancel="emit('track-pointer-up', $event)"
        @pointerleave="emit('track-pointer-leave')"
      >
        <!-- REGLA DE TIEMPO GRADUADA -->
        <TimelineRuler
          :timeline-track-width="timelineTrackWidth"
          :ruler-marks="rulerMarks"
        />

        <!-- CONTENEDOR DE PISTAS (V1 + A1) -->
        <div class="tracks-container">
          <!-- PISTA DE VIDEO (V1) -->
          <div class="track-row video-track">
            <div class="track-name-badge">
              <span class="material-symbols-outlined">videocam</span>
              <span>V1</span>
            </div>

            <!-- Huecos cortados / brechas eliminadas -->
            <div
              v-for="(gap, gIdx) in timelineGaps"
              :key="'v-gap-' + gIdx"
              class="track-gap-dimmed"
              :style="{
                left: `${gap.start * pixelsPerSecond}px`,
                width: `${(gap.end - gap.start) * pixelsPerSecond}px`
              }"
              @click.stop="emit('close-gap', gap)"
              :title="`Hueco cortado: ${formatTimecode(gap.end - gap.start, metadata.fps)}. Clic para cerrar hueco (Ripple Delete)`"
            >
              <div class="gap-content">
                <span class="gap-cut-text">Cortado</span>
                <button type="button" class="btn-close-gap" @click.stop="emit('close-gap', gap)">
                  <span class="material-symbols-outlined">fast_rewind</span>
                  <span>Cerrar hueco</span>
                </button>
              </div>
            </div>

            <!-- Clips de Video -->
            <div
              v-for="clip in clips"
              :key="'v-clip-' + clip.id"
              class="clip-timeline-item video-clip-item"
              :class="{ 'is-selected': selectedClipId === clip.id }"
              :style="{
                left: `${clip.start * pixelsPerSecond}px`,
                width: `${Math.max(24, (clip.end - clip.start) * pixelsPerSecond)}px`,
                '--clip-color': clip.color
              }"
              @pointerdown="emit('clip-pointer-down', $event, clip)"
              @pointermove="emit('clip-pointer-move', $event, clip)"
              @pointerup="emit('clip-pointer-up', $event, clip)"
              @pointercancel="emit('clip-pointer-up', $event, clip)"
              @contextmenu.prevent.stop="emit('context-menu', $event, clip)"
              @dblclick.stop="emit('jump-to-clip-start', clip)"
            >
              <!-- Miniaturas de fotogramas del clip -->
              <div class="clip-filmstrip-strip">
                <div
                  v-for="tile in (clipTilesMap[clip.id] || [])"
                  :key="tile.index"
                  class="filmstrip-tile"
                  :style="{ left: `${tile.pixelLeft}px`, width: `${tile.width}px` }"
                >
                  <img
                    v-if="tile.image"
                    :src="tile.image"
                    class="thumbnail-img"
                    alt="Fotograma"
                    draggable="false"
                  />
                  <div v-else class="tile-placeholder">
                    <span class="placeholder-time">{{ formatTimecode(tile.time, metadata.fps) }}</span>
                  </div>
                </div>
              </div>

              <div class="clip-tint-overlay"></div>

              <!-- Insignia del clip dentro del bloque -->
              <div class="clip-badge">
                <span class="clip-badge-dot" :style="{ background: clip.color }"></span>
                <span class="clip-badge-title">{{ clip.name }}</span>
                <span class="clip-badge-dur">{{ formatTimecode(clip.end - clip.start, metadata.fps) }}</span>
              </div>

              <!-- Marco de selección con asas laterales -->
              <div v-if="selectedClipId === clip.id" class="selection-frame">
                <div
                  class="handle handle-left"
                  @pointerdown.stop.prevent="emit('trim-handle-down', { event: $event, edge: 'left', clip })"
                  title="Arrastra para recortar el inicio del clip"
                ></div>
                <div
                  class="handle handle-right"
                  @pointerdown.stop.prevent="emit('trim-handle-down', { event: $event, edge: 'right', clip })"
                  title="Arrastra para recortar el final del clip"
                ></div>
              </div>
            </div>

            <!-- Muescas de puntos de corte -->
            <div
              v-for="sp in splitPoints"
              :key="'sp-v-' + sp"
              class="cut-seam-notch"
              :style="{ left: `${sp * pixelsPerSecond}px` }"
            >
              <div class="cut-seam-icon">
                <span class="material-symbols-outlined">content_cut</span>
              </div>
              <div class="cut-seam-line"></div>
            </div>
          </div>

          <!-- PISTA DE AUDIO (A1) -->
          <div class="track-row audio-track">
            <div class="track-name-badge">
              <span class="material-symbols-outlined">graphic_eq</span>
              <span>A1</span>
            </div>

            <!-- Huecos en audio -->
            <div
              v-for="(gap, gIdx) in timelineGaps"
              :key="'a-gap-' + gIdx"
              class="track-gap-dimmed"
              :style="{
                left: `${gap.start * pixelsPerSecond}px`,
                width: `${(gap.end - gap.start) * pixelsPerSecond}px`
              }"
              @click.stop="emit('close-gap', gap)"
            ></div>

            <!-- Clips de Audio -->
            <div
              v-for="clip in clips"
              :key="'a-clip-' + clip.id"
              class="clip-timeline-item audio-clip-item"
              :class="{ 'is-selected': selectedClipId === clip.id }"
              :style="{
                left: `${clip.start * pixelsPerSecond}px`,
                width: `${Math.max(24, (clip.end - clip.start) * pixelsPerSecond)}px`,
                '--clip-color': clip.color
              }"
              @pointerdown="emit('clip-pointer-down', $event, clip)"
              @pointermove="emit('clip-pointer-move', $event, clip)"
              @pointerup="emit('clip-pointer-up', $event, clip)"
              @pointercancel="emit('clip-pointer-up', $event, clip)"
              @contextmenu.prevent.stop="emit('context-menu', $event, clip)"
              @dblclick.stop="emit('jump-to-clip-start', clip)"
            >
              <!-- Forma de onda visual -->
              <div class="clip-audio-inner">
                <img
                  v-if="audioWaveformUrl"
                  :src="audioWaveformUrl"
                  class="clip-waveform-img"
                  :style="{
                    left: `-${(clip.sourceStart !== undefined ? clip.sourceStart : clip.start) * pixelsPerSecond}px`,
                    width: `${Math.max(800, (duration || 10) * pixelsPerSecond)}px`
                  }"
                  alt="Audio"
                  draggable="false"
                />
                <svg
                  v-else
                  class="clip-waveform-svg"
                  preserveAspectRatio="none"
                  viewBox="0 0 1000 32"
                  :style="{
                    left: `-${(clip.sourceStart !== undefined ? clip.sourceStart : clip.start) * pixelsPerSecond}px`,
                    width: `${Math.max(800, (duration || 10) * pixelsPerSecond)}px`
                  }"
                >
                  <path
                    d="M0,16 Q15,4 25,16 T50,16 T75,6 T100,16 T125,2 T150,16 T175,10 T200,16 T225,4 T250,16 T275,1 T300,16 T325,12 T350,16 T375,2 T400,16 T425,6 T450,16 T475,3 T500,16 T525,10 T550,16 T575,5 T600,16 T625,14 T650,16 T675,3 T700,16 T725,8 T750,16 T775,12 T800,16 T825,6 T850,16 T875,10 T900,16 T925,4 T950,16 T975,14 T1000,16 L1000,32 L0,32 Z"
                    fill="currentColor"
                    opacity="0.3"
                  ></path>
                  <path
                    d="M0,16 Q15,8 25,16 T50,16 T75,10 T100,16 T125,6 T150,16 T175,12 T200,16 T225,8 T250,16 T275,4 T300,16 T325,14 T350,16 T375,3 T400,16 T425,9 T450,16 T475,5 T500,16 T525,12 T550,16 T575,7 T600,16 T625,14 T650,16 T675,6 T700,16 T725,10 T750,16 T775,13 T800,16 T825,9 T850,16 T875,12 T900,16 T925,7 T950,16 T975,15 T1000,16 L1000,16 L0,16 Z"
                    fill="currentColor"
                  ></path>
                </svg>
              </div>

              <div class="clip-audio-tint"></div>

              <div class="clip-badge">
                <span class="clip-badge-dot" :style="{ background: clip.color }"></span>
                <span class="clip-badge-title">{{ clip.name }} (Audio)</span>
              </div>

              <div v-if="selectedClipId === clip.id" class="selection-frame">
                <div
                  class="handle handle-left"
                  @pointerdown.stop.prevent="emit('trim-handle-down', { event: $event, edge: 'left', clip })"
                  title="Arrastra para recortar el inicio del clip"
                ></div>
                <div
                  class="handle handle-right"
                  @pointerdown.stop.prevent="emit('trim-handle-down', { event: $event, edge: 'right', clip })"
                  title="Arrastra para recortar el final del clip"
                ></div>
              </div>
            </div>

            <!-- Muescas en audio -->
            <div
              v-for="sp in splitPoints"
              :key="'sp-a-' + sp"
              class="cut-seam-notch"
              :style="{ left: `${sp * pixelsPerSecond}px` }"
            >
              <div class="cut-seam-line"></div>
            </div>
          </div>
        </div>

        <!-- PLAYHEAD Y HOVER NEEDLE -->
        <TimelinePlayhead
          :fps="metadata.fps"
          :active-tool="activeTool"
        />
      </div>
    </div>
  </footer>
</template>

<style scoped>
.timeline-dock {
  background-color: var(--surface-low);
  border-top: 1px solid var(--border-subtle);
  display: flex;
  flex-direction: column;
  user-select: none;
  z-index: 30;
}

/* Scroll Viewport */
.timeline-scroll-viewport {
  width: 100%;
  height: 210px;
  overflow-x: auto;
  overflow-y: hidden;
  background-color: var(--surface-dim);
  position: relative;
}

.timeline-track-canvas {
  height: 100%;
  position: relative;
  min-width: 100%;
  cursor: pointer;
}

.timeline-track-canvas.tool-cut {
  cursor: crosshair;
}

/* Tracks */
.tracks-container {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 6px 0;
}

.track-row {
  position: relative;
  height: 76px;
  background-color: var(--surface-low);
  border: 1px solid rgba(255, 255, 255, 0.04);
  overflow: hidden;
}

.audio-track {
  height: 60px;
  background-color: var(--surface-dim);
}

.track-name-badge {
  position: absolute;
  left: 8px;
  top: 8px;
  z-index: 25;
  background-color: rgba(10, 14, 22, 0.85);
  backdrop-filter: blur(8px);
  border: 1px solid var(--border-subtle);
  padding: 2px 6px;
  border-radius: var(--radius-xs);
  display: flex;
  align-items: center;
  gap: 4px;
  font-family: var(--font-mono);
  font-size: 10px;
  font-weight: 700;
  color: var(--text-secondary);
  pointer-events: none;
}

.track-name-badge .material-symbols-outlined {
  font-size: 12px;
}

/* Gaps */
.track-gap-dimmed {
  position: absolute;
  top: 0;
  bottom: 0;
  background: repeating-linear-gradient(
    45deg,
    rgba(0, 0, 0, 0.5),
    rgba(0, 0, 0, 0.5) 10px,
    rgba(0, 0, 0, 0.7) 10px,
    rgba(0, 0, 0, 0.7) 20px
  );
  border: 1px dashed rgba(239, 68, 68, 0.25);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10;
  cursor: pointer;
}

.gap-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.gap-cut-text {
  font-family: var(--font-mono);
  font-size: 9px;
  color: var(--accent-red);
  text-transform: uppercase;
}

.btn-close-gap {
  display: flex;
  align-items: center;
  gap: 4px;
  background-color: rgba(239, 68, 68, 0.2);
  border: 1px solid rgba(239, 68, 68, 0.4);
  color: #ffdad6;
  padding: 2px 6px;
  border-radius: 3px;
  font-size: 9px;
  cursor: pointer;
}

.btn-close-gap .material-symbols-outlined {
  font-size: 12px;
}

/* Clips on timeline */
.clip-timeline-item {
  position: absolute;
  top: 2px;
  bottom: 2px;
  border-radius: var(--radius-sm);
  background-color: var(--surface-mid);
  overflow: hidden;
  z-index: 15;
  border: 1px solid rgba(255, 255, 255, 0.08);
}

.clip-timeline-item.is-selected {
  border-color: var(--accent-cyan);
  box-shadow: 0 0 12px rgba(6, 182, 212, 0.3);
}

.clip-filmstrip-strip {
  position: absolute;
  inset: 0;
  display: flex;
}

.filmstrip-tile {
  position: absolute;
  top: 0;
  bottom: 0;
  overflow: hidden;
}

.thumbnail-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  filter: brightness(0.85);
  user-select: none;
  -webkit-user-drag: none;
  pointer-events: none;
}

.tile-placeholder {
  width: 100%;
  height: 100%;
  background-color: var(--surface-mid);
  display: flex;
  align-items: center;
  justify-content: center;
}

.placeholder-time {
  font-family: var(--font-mono);
  font-size: 9px;
  color: var(--text-muted);
}

.clip-tint-overlay {
  position: absolute;
  inset: 0;
  background-color: rgba(6, 182, 212, 0.12);
  pointer-events: none;
}

.clip-badge {
  position: absolute;
  top: 4px;
  left: 6px;
  z-index: 5;
  display: flex;
  align-items: center;
  gap: 5px;
  background-color: rgba(10, 14, 22, 0.8);
  backdrop-filter: blur(6px);
  padding: 2px 6px;
  border-radius: 3px;
  font-family: var(--font-mono);
  font-size: 10px;
  pointer-events: none;
}

.clip-badge-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}

.clip-badge-title {
  color: var(--text-pure);
  font-weight: 600;
}

.clip-badge-dur {
  color: var(--accent-cyan-bright);
}

/* Audio waveform */
.clip-audio-inner {
  position: absolute;
  inset: 0;
  overflow: hidden;
}

.clip-waveform-img {
  position: absolute;
  top: 0;
  bottom: 0;
  object-fit: cover;
  opacity: 0.9;
  user-select: none;
  -webkit-user-drag: none;
  pointer-events: none;
}

.clip-waveform-svg {
  position: absolute;
  top: 0;
  bottom: 0;
  height: 100%;
  color: var(--accent-indigo);
}

.clip-audio-tint {
  position: absolute;
  inset: 0;
  background-color: rgba(99, 102, 241, 0.12);
  pointer-events: none;
}

/* Selection frame */
.selection-frame {
  position: absolute;
  inset: 0;
  border: 2px solid var(--accent-cyan);
  border-radius: var(--radius-sm);
  pointer-events: none;
  z-index: 25;
}

.handle {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 10px;
  background-color: var(--accent-cyan);
  pointer-events: auto;
  cursor: ew-resize;
  transition: background-color 0.15s ease, width 0.15s ease, box-shadow 0.15s ease;
  z-index: 30;
  display: flex;
  align-items: center;
  justify-content: center;
}

.handle::after {
  content: '';
  width: 2px;
  height: 12px;
  background-color: rgba(0, 0, 0, 0.4);
  border-radius: 1px;
}

.handle:hover {
  background-color: var(--accent-cyan-bright);
  box-shadow: 0 0 8px var(--accent-cyan);
  width: 12px;
}

.handle-left { left: 0; border-radius: 2px 0 0 2px; }
.handle-right { right: 0; border-radius: 0 2px 2px 0; }

/* Cut seam */
.cut-seam-notch {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 1px;
  z-index: 20;
  pointer-events: none;
}

.cut-seam-line {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 1px;
  background-color: var(--accent-amber);
  box-shadow: 0 0 6px var(--accent-amber);
}

.cut-seam-icon {
  position: absolute;
  top: -12px;
  left: -8px;
  color: var(--accent-amber);
  font-size: 14px;
}

.cut-seam-icon .material-symbols-outlined {
  font-size: 16px;
}
</style>
