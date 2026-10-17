<script setup lang="ts">
import { computed, inject, type Ref } from 'vue';
import type { ActiveTool } from '../../types';
import { formatTimecode } from '../../utils/formatters';

const props = defineProps<{
  fps: number;
  activeTool: ActiveTool;
  playheadPixelPosition?: number;
  currentTime?: number;
  isHoveringTimeline?: boolean;
  hoverTime?: number | null;
  hoverPixel?: number;
}>();

const playheadState = inject<{
  currentTime: Ref<number>;
  playheadPixelPosition: Ref<number>;
  hoverTime: Ref<number | null>;
  hoverPixel: Ref<number>;
  isHoveringTimeline: Ref<boolean>;
} | null>('timelinePlayheadState', null);

const currentPos = computed(() => (playheadState ? playheadState.playheadPixelPosition.value : (props.playheadPixelPosition ?? 0)));
const curTime = computed(() => (playheadState ? playheadState.currentTime.value : (props.currentTime ?? 0)));
const isHovering = computed(() => (playheadState ? playheadState.isHoveringTimeline.value : (props.isHoveringTimeline ?? false)));
const hTime = computed(() => (playheadState ? playheadState.hoverTime.value : (props.hoverTime ?? null)));
const hPixel = computed(() => (playheadState ? playheadState.hoverPixel.value : (props.hoverPixel ?? 0)));
</script>

<template>
  <!-- 1. AGUJA DE HOVER (Cuchilla y Guía de tiempo) -->
  <div
    v-if="isHovering && hTime !== null"
    class="hover-needle"
    :class="{ 'mode-cut': activeTool === 'cut' }"
    :style="{ transform: `translate3d(${hPixel}px, 0, 0)` }"
  >
    <div class="hover-badge">
      <span v-if="activeTool === 'cut'" class="material-symbols-outlined hover-cut-icon">content_cut</span>
      <span>{{ formatTimecode(hTime, fps) }}</span>
    </div>
    <div class="hover-line"></div>
  </div>

  <!-- 2. CABEZAL TEMPORAL (PLAYHEAD) -->
  <div
    class="playhead-needle"
    :style="{ transform: `translate3d(${currentPos}px, 0, 0)` }"
  >
    <div class="needle-cap">
      <span>{{ formatTimecode(curTime, fps) }}</span>
    </div>
    <div class="needle-line"></div>
  </div>
</template>

<style scoped>
/* Hover needle */
.hover-needle {
  position: absolute;
  top: 0;
  bottom: 0;
  left: 0;
  width: 1px;
  z-index: 40;
  pointer-events: none;
  will-change: transform;
}

.hover-line {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 1px;
  background-color: rgba(255, 255, 255, 0.4);
}

.hover-needle.mode-cut .hover-line {
  background-color: var(--accent-amber);
  box-shadow: 0 0 6px var(--accent-amber);
}

.hover-badge {
  position: absolute;
  top: 26px;
  left: 6px;
  background-color: rgba(10, 14, 22, 0.9);
  border: 1px solid var(--border-strong);
  padding: 2px 6px;
  border-radius: 3px;
  font-family: var(--font-mono);
  font-size: 9px;
  color: var(--text-pure);
  display: flex;
  align-items: center;
  gap: 4px;
  white-space: nowrap;
}

.hover-cut-icon {
  font-size: 11px;
  color: var(--accent-amber);
}

/* Playhead */
.playhead-needle {
  position: absolute;
  top: 0;
  bottom: 0;
  left: 0;
  width: 1px;
  z-index: 50;
  pointer-events: none;
  will-change: transform;
}

.needle-line {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 1.5px;
  background-color: var(--accent-cyan);
  box-shadow: 0 0 8px var(--accent-cyan);
}

.needle-cap {
  position: absolute;
  top: 0;
  left: 50%;
  transform: translateX(-50%);
  background-color: var(--accent-cyan);
  color: #04131f;
  font-family: var(--font-mono);
  font-size: 9px;
  font-weight: 700;
  padding: 2px 6px;
  border-radius: 3px;
  box-shadow: 0 2px 8px rgba(6, 182, 212, 0.6);
  white-space: nowrap;
}
</style>
