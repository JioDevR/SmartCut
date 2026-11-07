<script setup lang="ts">
import { ref } from 'vue';
import PlayerEmptyState from './PlayerEmptyState.vue';

const props = defineProps<{
  videoSrc: string;
  isPlaying: boolean;
  isPlayheadInGap: boolean;
}>();

const emit = defineEmits<{
  (e: 'open-file'): void;
  (e: 'toggle-play'): void;
  (e: 'time-update'): void;
  (e: 'loaded-metadata'): void;
  (e: 'loaded-data'): void;
  (e: 'seeked'): void;
  (e: 'error', evt: Event): void;
  (e: 'play-started'): void;
  (e: 'play-paused'): void;
}>();

const videoElement = ref<HTMLVideoElement | null>(null);

defineExpose({
  videoElement,
});
</script>

<template>
  <main class="player-stage">
    <!-- 1. Pantalla de bienvenida cuando no hay video cargado -->
    <PlayerEmptyState
      v-if="!videoSrc"
      @open-file="emit('open-file')"
    />

    <!-- 2. Viewport del monitor minimalista -->
    <div v-else class="viewport-wrapper">
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

        <!-- Marca central de reproducción (icono grande circular ▶ en el centro cuando está en pausa) -->
        <div class="center-play-watermark" v-if="!isPlaying" @click="emit('toggle-play')">
          <div class="watermark-circle">
            <span class="material-symbols-outlined">play_arrow</span>
          </div>
        </div>
      </div>
    </div>
  </main>
</template>

<style scoped>
.player-stage {
  flex: 1;
  background-color: #07090e;
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  user-select: none;
}

.viewport-wrapper {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
}

.video-container {
  position: relative;
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: #000000;
  overflow: hidden;
}

.main-video {
  max-width: 100%;
  max-height: 100%;
  width: auto;
  height: auto;
  object-fit: contain;
  cursor: pointer;
  display: block;
}

.video-hidden-gap {
  opacity: 0.15;
  filter: grayscale(1);
}

.monitor-empty-gap-screen {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: rgba(5, 7, 12, 0.92);
  cursor: pointer;
  z-index: 10;
}

.empty-gap-notice {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  background-color: rgba(239, 68, 68, 0.15);
  border: 1px solid rgba(239, 68, 68, 0.3);
  border-radius: var(--radius-sm);
  color: #fca5a5;
  font-family: var(--font-ui);
  font-size: 13px;
  font-weight: 500;
}

/* Marca de agua de Play central (exacta a la de la captura) */
.center-play-watermark {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  pointer-events: auto;
  cursor: pointer;
  z-index: 15;
}

.watermark-circle {
  width: 64px;
  height: 64px;
  border-radius: 50%;
  background-color: rgba(15, 23, 42, 0.65);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  border: 1px solid rgba(255, 255, 255, 0.15);
  display: flex;
  align-items: center;
  justify-content: center;
  color: rgba(255, 255, 255, 0.85);
  transition: all 0.2s cubic-bezier(0.2, 0, 0, 1);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
}

.watermark-circle .material-symbols-outlined {
  font-size: 32px;
  margin-left: 3px;
}

.center-play-watermark:hover .watermark-circle {
  background-color: #10b981;
  color: #000000;
  border-color: #34d399;
  transform: scale(1.08);
  box-shadow: 0 0 24px rgba(16, 185, 129, 0.5);
}
</style>
