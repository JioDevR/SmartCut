<script setup lang="ts">
import { ref } from 'vue';
import type { VideoMetadata, SampleVideoInfo, AspectRatio } from '../../types';
import PlayerEmptyState from './PlayerEmptyState.vue';
import PlayerControls from './PlayerControls.vue';

const props = defineProps<{
  videoSrc: string;
  isPlaying: boolean;
  isBuffering: boolean;
  currentTime: number;
  duration: number;
  volume: number;
  isMuted: boolean;
  playbackRate: number;
  showGuides: boolean;
  aspectRatio: AspectRatio;
  metadata: VideoMetadata;
  sampleVideos: SampleVideoInfo[];
  inPoint: number;
  outPoint: number;
  isPlayheadInGap: boolean;
}>();

const emit = defineEmits<{
  (e: 'open-file'): void;
  (e: 'load-sample', path: string): void;
  (e: 'toggle-play'): void;
  (e: 'step-frame', delta: number): void;
  (e: 'step-seconds', delta: number): void;
  (e: 'mark-in'): void;
  (e: 'mark-out'): void;
  (e: 'toggle-mute'): void;
  (e: 'set-volume', val: number): void;
  (e: 'set-aspect-ratio', ratio: AspectRatio): void;
  (e: 'toggle-guides'): void;
  (e: 'time-update'): void;
  (e: 'loaded-metadata'): void;
  (e: 'loaded-data'): void;
  (e: 'seeked'): void;
  (e: 'error', evt: Event): void;
  (e: 'play-started'): void;
  (e: 'play-paused'): void;
}>();

const videoElement = ref<HTMLVideoElement | null>(null);

function requestFullscreen() {
  const container = videoElement.value?.parentElement;
  if (!container) return;
  if (!document.fullscreenElement) {
    container.requestFullscreen().catch((err) => console.warn(err));
  } else {
    document.exitFullscreen().catch((err) => console.warn(err));
  }
}

defineExpose({
  videoElement,
});
</script>

<template>
  <main class="player-stage">
    <!-- 1. Pantalla de bienvenida cuando no hay video cargado -->
    <PlayerEmptyState
      v-if="!videoSrc"
      :sample-videos="sampleVideos"
      @open-file="emit('open-file')"
      @load-sample="emit('load-sample', $event)"
    />

    <!-- 2. Viewport del monitor con aspecto dinámico y HUD profesional -->
    <div v-else class="viewport-wrapper" :class="`aspect-${aspectRatio.replace(':', '-')}`">
      <div class="video-container group">
        <video
          ref="videoElement"
          :src="videoSrc"
          class="main-video"
          :class="{ 'video-hidden-gap': isPlayheadInGap && !isPlaying }"
          crossorigin="anonymous"
          playsinline
          preload="auto"
          @timeupdate="emit('time-update')"
          @loadedmetadata="emit('loaded-metadata')"
          @loadeddata="emit('loaded-data')"
          @seeked="emit('seeked')"
          @error="emit('error', $event)"
          @play="emit('play-started')"
          @pause="emit('play-paused')"
          @click="emit('toggle-play')"
        ></video>

        <!-- Pantalla de corte / vacío cuando el cabezal está sobre un hueco -->
        <div v-if="isPlayheadInGap" class="monitor-empty-gap-screen" @click="emit('toggle-play')">
          <div class="empty-gap-notice">
            <span class="material-symbols-outlined">videocam_off</span>
            <span>Espacio cortado / Vacío</span>
          </div>
        </div>

        <!-- Guías de Encuadre (Regla de Tercios) -->
        <div v-if="showGuides" class="framing-guides pointer-events-none">
          <div class="guide-v guide-v-1"></div>
          <div class="guide-v guide-v-2"></div>
          <div class="guide-h guide-h-1"></div>
          <div class="guide-h guide-h-2"></div>
        </div>

        <!-- Marca central de reproducción (Hover) -->
        <div class="center-play-watermark" v-if="!isPlaying" @click="emit('toggle-play')">
          <div class="watermark-circle">
            <span class="material-symbols-outlined">play_arrow</span>
          </div>
        </div>

        <!-- Controles flotantes HUD Top & Bottom -->
        <PlayerControls
          :current-time="currentTime"
          :duration="duration"
          :in-point="inPoint"
          :out-point="outPoint"
          :is-playing="isPlaying"
          :volume="volume"
          :is-muted="isMuted"
          :metadata="metadata"
          :aspect-ratio="aspectRatio"
          :show-guides="showGuides"
          @toggle-play="emit('toggle-play')"
          @step-frame="emit('step-frame', $event)"
          @step-seconds="emit('step-seconds', $event)"
          @mark-in="emit('mark-in')"
          @mark-out="emit('mark-out')"
          @toggle-mute="emit('toggle-mute')"
          @set-volume="emit('set-volume', $event)"
          @set-aspect-ratio="emit('set-aspect-ratio', $event)"
          @toggle-guides="emit('toggle-guides')"
          @request-fullscreen="requestFullscreen"
        />
      </div>
    </div>
  </main>
</template>

<style scoped>
.player-stage {
  flex: 1;
  background-color: var(--bg-canvas);
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  user-select: none;
}

/* Cinema Viewport */
.viewport-wrapper {
  position: relative;
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 12px;
  overflow: hidden;
  box-sizing: border-box;
  contain: layout size;
}

.video-container {
  position: relative;
  width: 100%;
  height: 100%;
  max-width: 100%;
  max-height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: #000000;
  border-radius: var(--radius-md);
  overflow: hidden;
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.7);
  contain: strict;
}

.aspect-16-9 .video-container { aspect-ratio: 16 / 9; }
.aspect-9-16 .video-container { aspect-ratio: 9 / 16; }
.aspect-1-1 .video-container { aspect-ratio: 1 / 1; }

.main-video {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: contain;
  background-color: #000000;
  transform: translateZ(0);
}

.video-hidden-gap {
  opacity: 0;
}

.monitor-empty-gap-screen {
  position: absolute;
  inset: 0;
  background-color: #000000;
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10;
  cursor: pointer;
}

.empty-gap-notice {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  color: var(--text-muted);
  font-size: 13px;
}

.empty-gap-notice .material-symbols-outlined {
  font-size: 36px;
  color: var(--accent-amber);
}

/* Framing guides */
.framing-guides {
  position: absolute;
  inset: 0;
  pointer-events: none;
  z-index: 15;
}

.guide-v {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 1px;
  background-color: rgba(255, 255, 255, 0.15);
}

.guide-v-1 { left: 33.333%; }
.guide-v-2 { left: 66.666%; }

.guide-h {
  position: absolute;
  left: 0;
  right: 0;
  height: 1px;
  background-color: rgba(255, 255, 255, 0.15);
}

.guide-h-1 { top: 33.333%; }
.guide-h-2 { top: 66.666%; }

/* Center play watermark */
.center-play-watermark {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.2s;
}

.video-container:hover .center-play-watermark {
  opacity: 1;
  pointer-events: auto;
}

.watermark-circle {
  width: 64px;
  height: 64px;
  border-radius: 50%;
  background-color: rgba(10, 14, 22, 0.65);
  backdrop-filter: blur(10px);
  border: 1px solid rgba(255, 255, 255, 0.15);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--accent-cyan);
  box-shadow: 0 8px 30px rgba(0, 0, 0, 0.6);
  cursor: pointer;
  transition: transform 0.15s;
}

.watermark-circle:hover {
  transform: scale(1.08);
}

.watermark-circle .material-symbols-outlined {
  font-size: 34px;
  margin-left: 3px;
}
</style>
