<script setup lang="ts">
import { computed, ref } from 'vue';
import type { ClipItem, TimelineGap, TimelineTile, RulerMark, ActiveTool, VideoMetadata } from '../../types';
import { formatTimecode } from '../../utils/formatters';
import TimelineToolbar from './TimelineToolbar.vue';
import TimelineRuler from './TimelineRuler.vue';
import TimelinePlayhead from './TimelinePlayhead.vue';
import AudioClipWaveform from './AudioClipWaveform.vue';

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
  audioPeaks?: Uint8Array | null;
  scrollLeft?: number;
  viewportWidth?: number;
  metadata: VideoMetadata;
  rulerMarks: RulerMark[];
  timelineTrackWidth: number;
  isPlaying: boolean;
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
  (e: 'toggle-play'): void;
  (e: 'step-frame', delta: number): void;
  (e: 'step-seconds', delta: number): void;
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
    <!-- 1. BARRA DE HERRAMIENTAS CENTRAL (Edición + Transporte + Zoom/Timecode) -->
    <TimelineToolbar
      :active-tool="activeTool"
      :can-undo="canUndo"
      :can-redo="canRedo"
      :is-playing="isPlaying"
      :pixels-per-second="pixelsPerSecond"
      :min-pixels-per-second="minPixelsPerSecond"
      :max-pixels-per-second="maxPixelsPerSecond"
      :duration="duration"
      :metadata="metadata"
      @set-tool="emit('set-tool', $event)"
      @split-at-playhead="emit('split-at-playhead')"
      @undo="emit('undo')"
      @redo="emit('redo')"
      @toggle-play="emit('toggle-play')"
      @step-frame="emit('step-frame', $event)"
      @step-seconds="emit('step-seconds', $event)"
      @zoom-in="emit('zoom-in')"
      @zoom-out="emit('zoom-out')"
      @set-zoom="emit('set-zoom', $event)"
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
            <!-- Huecos cortados / brechas limpias sin banners agresivos -->
            <div
              v-for="(gap, gIdx) in timelineGaps"
              :key="'v-gap-' + gIdx"
              class="track-gap-clean"
              :style="{
                left: `${gap.start * pixelsPerSecond}px`,
                width: `${(gap.end - gap.start) * pixelsPerSecond}px`
              }"
              @click.stop="emit('close-gap', gap)"
              :title="`Hueco cortado (${formatTimecode(gap.end - gap.start, metadata.fps)}). Clic para unir.`"
            ></div>

            <!-- Clips de Video -->
            <div
              v-for="clip in clips"
              :key="'v-clip-' + clip.id"
              class="clip-timeline-item video-clip-item"
              :class="{ 'is-selected': selectedClipId === clip.id }"
              :style="{
                left: `${clip.start * pixelsPerSecond}px`,
                width: `${Math.max(24, (clip.end - clip.start) * pixelsPerSecond)}px`
              }"
              @pointerdown="emit('clip-pointer-down', $event, clip)"
              @pointermove="emit('clip-pointer-move', $event, clip)"
              @pointerup="emit('clip-pointer-up', $event, clip)"
              @pointercancel="emit('clip-pointer-up', $event, clip)"
              @contextmenu.prevent.stop="emit('context-menu', $event, clip)"
              @dblclick.stop="emit('jump-to-clip-start', clip)"
            >
              <!-- Insignia interna de cabecera V1 (Exacta a la captura) -->
              <div class="track-inner-badge-v1">
                <div class="tib-top">
                  <span class="material-symbols-outlined tib-icon">videocam</span>
                  <span class="tib-name">V1</span>
                  <span class="tib-tc">{{ formatTimecode(clip.end - clip.start, metadata.fps) }}</span>
                </div>
                <div class="tib-sub">V1 START</div>
              </div>

              <!-- Miniaturas / Marcadores de fotogramas del clip -->
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
                    alt=""
                    draggable="false"
                    loading="lazy"
                    @error="(e) => { const el = e.target as HTMLElement; if (el) el.style.opacity = '0'; }"
                  />
                  <div v-else class="tile-placeholder">
                    <span class="placeholder-time">{{ formatTimecode(tile.time, metadata.fps) }}</span>
                  </div>
                </div>
              </div>

              <!-- Tarjeta central de fotograma clave (Exacta a la captura) -->
              <div
                class="keyframe-center-card"
                :style="{
                  left: '50%',
                  transform: 'translateX(-50%)'
                }"
              >
                <span class="material-symbols-outlined kf-icon">image</span>
                <span class="kf-tc">{{ formatTimecode((clip.start + clip.end) / 2, metadata.fps) }}</span>
              </div>

              <!-- Marco de selección blanco de alto contraste (Exacto a la captura) -->
              <div v-if="selectedClipId === clip.id" class="selection-frame-white">
                <div
                  class="handle handle-left"
                  @pointerdown.stop.prevent="emit('trim-handle-down', { event: $event, edge: 'left', clip })"
                  title="Arrastra para recortar el inicio"
                ></div>
                <div
                  class="handle handle-right"
                  @pointerdown.stop.prevent="emit('trim-handle-down', { event: $event, edge: 'right', clip })"
                  title="Arrastra para recortar el final"
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
              <div class="cut-seam-line"></div>
            </div>
          </div>

          <!-- PISTA DE AUDIO (A1) -->
          <div class="track-row audio-track">
            <!-- Huecos en audio -->
            <div
              v-for="(gap, gIdx) in timelineGaps"
              :key="'a-gap-' + gIdx"
              class="track-gap-clean"
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
                width: `${Math.max(24, (clip.end - clip.start) * pixelsPerSecond)}px`
              }"
              @pointerdown="emit('clip-pointer-down', $event, clip)"
              @pointermove="emit('clip-pointer-move', $event, clip)"
              @pointerup="emit('clip-pointer-up', $event, clip)"
              @pointercancel="emit('clip-pointer-up', $event, clip)"
              @contextmenu.prevent.stop="emit('context-menu', $event, clip)"
              @dblclick.stop="emit('jump-to-clip-start', clip)"
            >
              <!-- Insignia interna A1 (Exacta a la captura: 🔊 A1 1 (Audio)) -->
              <div class="track-inner-badge-a1">
                <span class="material-symbols-outlined tib-icon">volume_up</span>
                <span class="tib-name">A1 1 (Audio)</span>
              </div>

              <!-- Onda de Audio Dinámica (Picos de Audio Reales en Canvas 60FPS) -->
              <div class="clip-audio-inner">
                <AudioClipWaveform
                  :clip="clip"
                  :pixels-per-second="pixelsPerSecond"
                  :audio-peaks="audioPeaks || null"
                  :scroll-left="scrollLeft || 0"
                  :viewport-width="viewportWidth || 1200"
                />
              </div>

              <!-- Marco de selección blanco de alto contraste -->
              <div v-if="selectedClipId === clip.id" class="selection-frame-white">
                <div
                  class="handle handle-left"
                  @pointerdown.stop.prevent="emit('trim-handle-down', { event: $event, edge: 'left', clip })"
                ></div>
                <div
                  class="handle handle-right"
                  @pointerdown.stop.prevent="emit('trim-handle-down', { event: $event, edge: 'right', clip })"
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
  background-color: #0b0f17;
  border-top: 1px solid var(--border-subtle);
  display: flex;
  flex-direction: column;
  user-select: none;
  z-index: 30;
}

/* Scroll Viewport */
.timeline-scroll-viewport {
  width: 100%;
  height: 220px;
  overflow-x: auto;
  overflow-y: hidden;
  background-color: #080b11;
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
  gap: 6px;
  padding: 8px 0;
}

.track-row {
  position: relative;
  height: 78px;
  background-color: rgba(14, 18, 26, 0.7);
  border: 1px solid rgba(255, 255, 255, 0.04);
  overflow: hidden;
}

.audio-track {
  height: 56px;
  background-color: rgba(12, 16, 23, 0.8);
}

/* Insignias integradas dentro del clip (como en la captura) */
.track-inner-badge-v1 {
  position: absolute;
  top: 6px;
  left: 8px;
  z-index: 25;
  background-color: rgba(9, 13, 20, 0.88);
  backdrop-filter: blur(8px);
  border: 1px solid rgba(255, 255, 255, 0.12);
  padding: 3px 8px;
  border-radius: 4px;
  display: flex;
  flex-direction: column;
  gap: 1px;
  pointer-events: none;
}

.tib-top {
  display: flex;
  align-items: center;
  gap: 4px;
}

.tib-icon {
  font-size: 13px;
  color: #10b981;
}

.tib-name {
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 700;
  color: #ffffff;
}

.tib-tc {
  font-family: var(--font-mono);
  font-size: 10px;
  color: #10b981;
  margin-left: 2px;
}

.tib-sub {
  font-family: var(--font-mono);
  font-size: 8px;
  color: var(--text-muted);
  letter-spacing: 0.05em;
}

.track-inner-badge-a1 {
  position: absolute;
  top: 6px;
  left: 8px;
  z-index: 25;
  background-color: rgba(9, 13, 20, 0.88);
  backdrop-filter: blur(8px);
  border: 1px solid rgba(255, 255, 255, 0.12);
  padding: 3px 8px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  gap: 5px;
  pointer-events: none;
}

.track-inner-badge-a1 .tib-name {
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 600;
  color: #10b981;
}

/* Huecos limpios */
.track-gap-clean {
  position: absolute;
  top: 0;
  bottom: 0;
  background-color: rgba(0, 0, 0, 0.5);
  border: 1px dashed rgba(255, 255, 255, 0.06);
  z-index: 10;
  cursor: pointer;
}

/* Clips on timeline */
.clip-timeline-item {
  position: absolute;
  top: 2px;
  bottom: 2px;
  border-radius: 4px;
  background-color: rgba(18, 24, 34, 0.9);
  overflow: hidden;
  z-index: 15;
  border: 1px solid rgba(255, 255, 255, 0.12);
}

.video-clip-item {
  background-color: rgba(15, 20, 30, 0.95);
}

.audio-clip-item {
  background: linear-gradient(180deg, rgba(16, 22, 34, 0.9), rgba(10, 14, 22, 0.95));
}

/* Filmstrip */
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
  border-right: 1px solid rgba(0, 0, 0, 0.35);
}

.thumbnail-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  filter: brightness(0.8);
  user-select: none;
  -webkit-user-drag: none;
  pointer-events: none;
  transition: opacity 0.2s ease-in;
}

.tile-placeholder {
  width: 100%;
  height: 100%;
  background-color: rgba(22, 29, 41, 0.6);
  display: flex;
  align-items: flex-end;
  justify-content: flex-end;
  padding: 4px 6px;
}

.placeholder-time {
  font-family: var(--font-mono);
  font-size: 9px;
  color: rgba(255, 255, 255, 0.35);
}

/* Tarjeta central de fotograma clave (Exacta a la captura) */
.keyframe-center-card {
  position: absolute;
  top: 10px;
  z-index: 20;
  background-color: rgba(10, 14, 22, 0.85);
  backdrop-filter: blur(8px);
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 4px;
  padding: 4px 8px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  pointer-events: none;
}

.kf-icon {
  font-size: 16px;
  color: var(--text-secondary);
}

.kf-tc {
  font-family: var(--font-mono);
  font-size: 9px;
  color: var(--text-secondary);
}

/* Onda sinusoidal estilizada para audio */
.clip-audio-inner {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.clip-sinusoidal-svg {
  width: 100%;
  height: 100%;
}

/* Marco de selección blanco de alto contraste (Exacto a la captura) */
.selection-frame-white {
  position: absolute;
  inset: 0;
  border: 2px solid #ffffff;
  border-radius: 4px;
  pointer-events: none;
  z-index: 30;
  box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.5), inset 0 0 0 1px rgba(0, 0, 0, 0.5);
}

.handle {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 10px;
  background-color: #ffffff;
  pointer-events: auto;
  cursor: ew-resize;
  transition: all 0.15s ease;
  z-index: 35;
}

.handle:hover {
  background-color: #10b981;
  box-shadow: 0 0 8px rgba(16, 185, 129, 0.7);
  width: 12px;
}

.handle-left {
  left: -2px;
  border-radius: 3px 0 0 3px;
}

.handle-right {
  right: -2px;
  border-radius: 0 3px 3px 0;
}

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
  background-color: #10b981;
  box-shadow: 0 0 6px rgba(16, 185, 129, 0.7);
}
</style>
