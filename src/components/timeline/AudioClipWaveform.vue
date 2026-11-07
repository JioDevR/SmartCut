<script setup lang="ts">
import { ref, computed, watch, onMounted, nextTick } from 'vue';
import type { ClipItem } from '../../types';

const props = defineProps<{
  clip: ClipItem;
  pixelsPerSecond: number;
  audioPeaks: Uint8Array | null;
  scrollLeft: number;
  viewportWidth: number;
}>();

const CHUNK_SIZE = 1600;
const canvasMap = new Map<number, HTMLCanvasElement>();

const clipWidth = computed(() => {
  const dur = Math.max(0, props.clip.end - props.clip.start);
  return Math.max(24, Math.floor(dur * props.pixelsPerSecond));
});

interface ChunkInfo {
  id: number;
  left: number;
  width: number;
}

const allChunks = computed<ChunkInfo[]>(() => {
  const totalW = clipWidth.value;
  const chunks: ChunkInfo[] = [];
  let currentLeft = 0;
  let id = 0;

  while (currentLeft < totalW) {
    const w = Math.min(CHUNK_SIZE, totalW - currentLeft);
    chunks.push({ id, left: currentLeft, width: w });
    currentLeft += w;
    id++;
  }

  return chunks;
});

const visibleChunks = computed(() => {
  const clipGlobalLeft = props.clip.start * props.pixelsPerSecond;
  const sLeft = props.scrollLeft - 300;
  const sRight = props.scrollLeft + props.viewportWidth + 300;

  return allChunks.value.filter((chunk) => {
    const chunkGlobalLeft = clipGlobalLeft + chunk.left;
    const chunkGlobalRight = chunkGlobalLeft + chunk.width;
    return chunkGlobalRight >= sLeft && chunkGlobalLeft <= sRight;
  });
});

function setCanvasRef(id: number, el: any) {
  if (el) {
    canvasMap.set(id, el as HTMLCanvasElement);
    const chunk = allChunks.value.find((c) => c.id === id);
    if (chunk) {
      drawChunk(el as HTMLCanvasElement, chunk);
    }
  } else {
    canvasMap.delete(id);
  }
}

function drawChunk(canvas: HTMLCanvasElement, chunk: ChunkInfo) {
  const ctx = canvas.getContext('2d');
  if (!ctx) return;

  const dpr = window.devicePixelRatio || 1;
  const w = chunk.width;
  const h = 52;

  canvas.width = Math.floor(w * dpr);
  canvas.height = Math.floor(h * dpr);

  ctx.save();
  ctx.scale(dpr, dpr);
  ctx.clearRect(0, 0, w, h);

  const midY = h / 2;

  // Línea base central sutil de referencia
  ctx.fillStyle = 'rgba(16, 185, 129, 0.25)';
  ctx.fillRect(0, midY, w, 1);

  const peaks = props.audioPeaks;
  if (!peaks || peaks.length === 0) {
    // Onda placeholder decorativa mientras se procesan los picos de audio
    ctx.fillStyle = 'rgba(16, 185, 129, 0.3)';
    for (let x = 0; x < w; x += 4) {
      const fakeAmp = Math.sin((chunk.left + x) * 0.04) * 6 + 8;
      ctx.fillRect(x, midY - fakeAmp / 2, 2, fakeAmp);
    }
    ctx.restore();
    return;
  }

  const srcStart = props.clip.sourceStart !== undefined ? props.clip.sourceStart : props.clip.start;
  const srcEnd = props.clip.sourceEnd !== undefined ? props.clip.sourceEnd : props.clip.end;
  const srcDur = Math.max(0.01, srcEnd - srcStart);
  const totalClipWidth = clipWidth.value;

  // Gradiente vertical verde esmeralda sutil de alta precisión
  const grad = ctx.createLinearGradient(0, 4, 0, h - 4);
  grad.addColorStop(0, '#34d399');
  grad.addColorStop(0.5, '#10b981');
  grad.addColorStop(1, '#059669');
  ctx.fillStyle = grad;

  const peaksLen = peaks.length;

  for (let x = 0; x < w; x++) {
    const pxInClip = chunk.left + x;
    const progress1 = pxInClip / totalClipWidth;
    const progress2 = (pxInClip + 1) / totalClipWidth;
    const t1 = srcStart + progress1 * srcDur;
    const t2 = srcStart + progress2 * srcDur;

    const idx1 = Math.max(0, Math.floor(t1 * 100));
    const idx2 = Math.max(idx1, Math.min(peaksLen - 1, Math.floor(t2 * 100)));

    let maxPeak = 0;
    for (let i = idx1; i <= idx2; i++) {
      if (peaks[i] > maxPeak) {
        maxPeak = peaks[i];
      }
    }

    if (maxPeak > 0) {
      const norm = maxPeak / 255;
      const barHeight = Math.max(2, Math.round(norm * (midY - 4) * 2));
      ctx.fillRect(x, midY - barHeight / 2, 1, barHeight);
    }
  }

  ctx.restore();
}

function redrawAll() {
  nextTick(() => {
    for (const chunk of visibleChunks.value) {
      const canvas = canvasMap.get(chunk.id);
      if (canvas) {
        drawChunk(canvas, chunk);
      }
    }
  });
}

watch(() => props.audioPeaks, redrawAll);
watch(() => props.pixelsPerSecond, redrawAll);
watch(() => [props.clip.start, props.clip.end, props.clip.sourceStart, props.clip.sourceEnd], redrawAll);
watch(() => visibleChunks.value.map((c) => c.id).join(','), redrawAll);

onMounted(() => {
  redrawAll();
});
</script>

<template>
  <div class="audio-clip-waveform-root">
    <canvas
      v-for="chunk in visibleChunks"
      :key="chunk.id"
      :ref="(el) => setCanvasRef(chunk.id, el)"
      class="audio-waveform-chunk"
      :style="{
        left: `${chunk.left}px`,
        width: `${chunk.width}px`
      }"
    />
  </div>
</template>

<style scoped>
.audio-clip-waveform-root {
  position: absolute;
  inset: 0;
  pointer-events: none;
  overflow: hidden;
}

.audio-waveform-chunk {
  position: absolute;
  top: 0;
  height: 100%;
  image-rendering: pixelated;
}
</style>
