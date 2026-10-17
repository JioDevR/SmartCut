<script setup lang="ts">
import { ref, watch, onMounted, nextTick } from 'vue';
import type { RulerMark } from '../../types';

const props = defineProps<{
  timelineTrackWidth: number;
  rulerMarks: RulerMark[];
}>();

const rulerCanvasRef = ref<HTMLCanvasElement | null>(null);

function renderRulerCanvas() {
  const canvas = rulerCanvasRef.value;
  if (!canvas) return;
  const ctx = canvas.getContext('2d');
  if (!ctx) return;

  const dpr = window.devicePixelRatio || 1;
  const trackWidth = props.timelineTrackWidth;
  const height = 24;

  const targetW = Math.max(1, Math.round(trackWidth * dpr));
  const targetH = Math.round(height * dpr);

  if (canvas.width !== targetW || canvas.height !== targetH) {
    canvas.width = targetW;
    canvas.height = targetH;
  }

  ctx.save();
  ctx.scale(dpr, dpr);
  ctx.clearRect(0, 0, trackWidth, height);

  ctx.font = '9px "JetBrains Mono", monospace, sans-serif';
  ctx.textBaseline = 'top';

  const marks = props.rulerMarks || [];
  for (let i = 0; i < marks.length; i++) {
    const mark = marks[i];
    const x = Math.round(mark.pixel);

    ctx.fillStyle = mark.isMajor ? 'rgba(255, 255, 255, 0.25)' : 'rgba(255, 255, 255, 0.08)';
    const tickH = mark.isMajor ? height : 6;
    ctx.fillRect(x, height - tickH, 1, tickH);

    if (mark.label) {
      ctx.fillStyle = 'rgba(255, 255, 255, 0.65)';
      ctx.fillText(mark.label, x + 4, 3);
    }
  }

  ctx.restore();
}

watch(
  () => [props.rulerMarks, props.timelineTrackWidth],
  () => {
    requestAnimationFrame(renderRulerCanvas);
  },
  { deep: false }
);

onMounted(() => {
  nextTick(renderRulerCanvas);
});
</script>

<template>
  <div class="timecode-ruler">
    <canvas
      ref="rulerCanvasRef"
      class="ruler-canvas"
      :style="{ width: `${timelineTrackWidth}px`, height: '24px' }"
    ></canvas>
  </div>
</template>

<style scoped>
.timecode-ruler {
  height: 24px;
  background-color: var(--surface-mid);
  border-bottom: 1px solid var(--border-subtle);
  position: relative;
  overflow: hidden;
}

.ruler-canvas {
  display: block;
  height: 24px;
  pointer-events: none;
}
</style>
