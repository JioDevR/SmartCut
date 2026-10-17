<script setup lang="ts">
import type { VideoMetadata, AspectRatio } from '../../types';
import { formatTimecode } from '../../utils/formatters';

const props = defineProps<{
  currentTime: number;
  duration: number;
  inPoint: number;
  outPoint: number;
  isPlaying: boolean;
  volume: number;
  isMuted: boolean;
  metadata: VideoMetadata;
  aspectRatio: AspectRatio;
  showGuides: boolean;
}>();

const emit = defineEmits<{
  (e: 'toggle-play'): void;
  (e: 'step-frame', delta: number): void;
  (e: 'step-seconds', delta: number): void;
  (e: 'mark-in'): void;
  (e: 'mark-out'): void;
  (e: 'toggle-mute'): void;
  (e: 'set-volume', val: number): void;
  (e: 'set-aspect-ratio', ratio: AspectRatio): void;
  (e: 'toggle-guides'): void;
  (e: 'request-fullscreen'): void;
}>();
</script>

<template>
  <!-- TOP FLOATING HUD (Obsidian Glass Pill) -->
  <div class="hud-top">
    <!-- Info técnica izquierda -->
    <div class="hud-pill">
      <span class="hud-dot"></span>
      <span class="hud-tech-text">{{ metadata.width }}x{{ metadata.height }}</span>
      <span class="hud-divider">/</span>
      <span class="hud-tech-muted">{{ metadata.fps ? metadata.fps.toFixed(2) : '30.00' }} FPS</span>
      <span class="hud-divider">/</span>
      <span class="hud-tech-cyan">TC {{ formatTimecode(currentTime, metadata.fps) }}</span>
    </div>

    <!-- Opciones de encuadre y pantalla derecha -->
    <div class="hud-pill hud-options">
      <button
        v-for="r in (['16:9', '9:16', '1:1'] as AspectRatio[])"
        :key="r"
        type="button"
        class="aspect-btn"
        :class="{ active: aspectRatio === r }"
        @click="emit('set-aspect-ratio', r)"
      >
        {{ r }}
      </button>
      <span class="hud-divider"></span>
      <button
        type="button"
        class="hud-icon-btn"
        :class="{ active: showGuides }"
        @click="emit('toggle-guides')"
        title="Guías de Tercios"
      >
        <span class="material-symbols-outlined">grid_4x4</span>
      </button>
      <button
        type="button"
        class="hud-icon-btn"
        @click="emit('request-fullscreen')"
        title="Pantalla Completa"
      >
        <span class="material-symbols-outlined">fullscreen</span>
      </button>
    </div>
  </div>

  <!-- BOTTOM FLOATING HUD ISLAND (Transporte de precisión) -->
  <div class="hud-bottom">
    <!-- Badge izquierdo: Puntos In / Out -->
    <div class="hud-pill hud-inout-badge">
      <div class="inout-tag in-tag">
        <span class="tag-title">IN</span>
        <span>{{ formatTimecode(inPoint, metadata.fps) }}</span>
      </div>
      <span class="inout-arrow">→</span>
      <div class="inout-tag out-tag">
        <span class="tag-title">OUT</span>
        <span>{{ formatTimecode(outPoint > 0 ? outPoint : duration, metadata.fps) }}</span>
      </div>
    </div>

    <!-- Isla central de transporte -->
    <div class="transport-island">
      <!-- Mark In [I] -->
      <button
        type="button"
        class="transport-btn btn-mark"
        @click="emit('mark-in')"
        title="Marcar Punto Entrada [I]"
      >
        <span class="kbd-label text-cyan">[I]</span>
        <span class="material-symbols-outlined">start</span>
      </button>

      <!-- -10s -->
      <button
        type="button"
        class="transport-btn transport-icon-btn"
        @click="emit('step-seconds', -10)"
        title="Retroceder 10s [J]"
      >
        <span class="material-symbols-outlined">replay_10</span>
      </button>

      <!-- -1 frame -->
      <button
        type="button"
        class="transport-btn transport-icon-btn"
        @click="emit('step-frame', -1)"
        title="Retroceder 1 Cuadro (Ctrl + Flecha Izq)"
      >
        <span class="material-symbols-outlined">skip_previous</span>
      </button>

      <!-- Play / Pause Principal con Brillo Cian -->
      <button
        type="button"
        class="master-play-btn"
        @click="emit('toggle-play')"
        :title="isPlaying ? 'Pausar [Espacio / K]' : 'Reproducir [Espacio / K]'"
      >
        <span class="material-symbols-outlined">
          {{ isPlaying ? 'pause' : 'play_arrow' }}
        </span>
      </button>

      <!-- +1 frame -->
      <button
        type="button"
        class="transport-btn transport-icon-btn"
        @click="emit('step-frame', 1)"
        title="Avanzar 1 Cuadro (Ctrl + Flecha Der)"
      >
        <span class="material-symbols-outlined">skip_next</span>
      </button>

      <!-- +10s -->
      <button
        type="button"
        class="transport-btn transport-icon-btn"
        @click="emit('step-seconds', 10)"
        title="Avanzar 10s [L]"
      >
        <span class="material-symbols-outlined">forward_10</span>
      </button>

      <!-- Mark Out [O] -->
      <button
        type="button"
        class="transport-btn btn-mark"
        @click="emit('mark-out')"
        title="Marcar Punto Salida [O]"
      >
        <span class="material-symbols-outlined">last_page</span>
        <span class="kbd-label text-amber">[O]</span>
      </button>
    </div>

    <!-- Control de volumen derecho -->
    <div class="hud-pill hud-volume-pill">
      <button
        type="button"
        class="hud-icon-btn"
        @click="emit('toggle-mute')"
        :title="isMuted ? 'Activar sonido' : 'Silenciar'"
      >
        <span class="material-symbols-outlined">
          {{ isMuted || volume === 0 ? 'volume_off' : (volume < 0.5 ? 'volume_down' : 'volume_up') }}
        </span>
      </button>
      <input
        type="range"
        min="0"
        max="1"
        step="0.05"
        class="volume-slider"
        :value="isMuted ? 0 : volume"
        @input="emit('set-volume', Number(($event.target as HTMLInputElement).value))"
      />
      <span class="volume-percent">{{ Math.round((isMuted ? 0 : volume) * 100) }}%</span>
    </div>
  </div>
</template>

<style scoped>
/* HUD Top */
.hud-top {
  position: absolute;
  top: 14px;
  left: 14px;
  right: 14px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  pointer-events: none;
  z-index: 20;
}

.hud-pill {
  display: flex;
  align-items: center;
  gap: 8px;
  background-color: rgba(10, 14, 22, 0.85);
  backdrop-filter: blur(14px);
  -webkit-backdrop-filter: blur(14px);
  border: 1px solid rgba(255, 255, 255, 0.08);
  padding: 4px 12px;
  border-radius: var(--radius-full);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.5);
  pointer-events: auto;
}

.hud-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background-color: var(--accent-cyan);
  box-shadow: 0 0 6px var(--accent-cyan);
}

.hud-tech-text {
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 500;
  color: var(--text-pure);
}

.hud-tech-muted {
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--text-secondary);
}

.hud-tech-cyan {
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--accent-cyan-bright);
  font-weight: 600;
}

.hud-divider {
  color: var(--text-muted);
  font-size: 10px;
}

.hud-options {
  gap: 4px;
  padding: 3px 6px;
}

.aspect-btn {
  background: none;
  border: none;
  color: var(--text-secondary);
  font-family: var(--font-mono);
  font-size: 10px;
  padding: 2px 6px;
  border-radius: var(--radius-full);
  cursor: pointer;
  transition: all 0.15s;
}

.aspect-btn:hover {
  color: var(--text-pure);
}

.aspect-btn.active {
  background-color: var(--surface-high);
  color: var(--accent-cyan);
  font-weight: 600;
}

.hud-icon-btn {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  background: none;
  border: none;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.15s;
}

.hud-icon-btn .material-symbols-outlined {
  font-size: 15px;
}

.hud-icon-btn:hover {
  color: var(--text-pure);
  background-color: var(--surface-high);
}

.hud-icon-btn.active {
  color: var(--accent-cyan);
}

/* HUD Bottom */
.hud-bottom {
  position: absolute;
  bottom: 14px;
  left: 14px;
  right: 14px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  pointer-events: none;
  z-index: 20;
}

/* In / Out badge */
.hud-inout-badge {
  display: flex;
  align-items: center;
  gap: 6px;
  font-family: var(--font-mono);
  font-size: 11px;
}

.inout-tag {
  display: flex;
  align-items: center;
  gap: 4px;
}

.tag-title {
  font-weight: 700;
  font-size: 10px;
}

.in-tag .tag-title {
  color: var(--accent-cyan);
}

.out-tag .tag-title {
  color: var(--accent-amber);
}

.inout-arrow {
  color: var(--text-muted);
}

/* Transport island */
.transport-island {
  display: flex;
  align-items: center;
  gap: 6px;
  background-color: rgba(10, 14, 22, 0.9);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(255, 255, 255, 0.1);
  padding: 4px 14px;
  border-radius: var(--radius-full);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.7);
  pointer-events: auto;
}

.transport-btn {
  background: none;
  border: none;
  color: var(--text-secondary);
  border-radius: var(--radius-full);
  cursor: pointer;
  transition: all 0.15s;
}

.transport-icon-btn {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.transport-icon-btn .material-symbols-outlined {
  font-size: 18px;
}

.transport-icon-btn:hover {
  color: var(--text-pure);
  background-color: var(--surface-high);
}

.btn-mark {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 3px 6px;
  font-size: 10px;
}

.btn-mark .material-symbols-outlined {
  font-size: 15px;
}

.btn-mark:hover {
  background-color: var(--surface-high);
  color: var(--text-pure);
}

.kbd-label {
  font-family: var(--font-mono);
  font-weight: 700;
  font-size: 10px;
}

.master-play-btn {
  width: 38px;
  height: 38px;
  border-radius: 50%;
  background: linear-gradient(135deg, #06b6d4, #0284c7);
  border: 1px solid rgba(76, 215, 246, 0.6);
  color: #04131f;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.15s;
  box-shadow: 0 0 16px rgba(6, 182, 212, 0.6);
}

.master-play-btn:hover {
  transform: scale(1.08);
  box-shadow: 0 0 22px rgba(6, 182, 212, 0.85);
}

.master-play-btn .material-symbols-outlined {
  font-size: 22px;
}

/* Volume pill */
.hud-volume-pill {
  display: flex;
  align-items: center;
  gap: 6px;
}

.volume-slider {
  width: 60px;
  height: 4px;
  appearance: none;
  background: var(--surface-highest);
  border-radius: 999px;
  outline: none;
  cursor: pointer;
}

.volume-slider::-webkit-slider-thumb {
  appearance: none;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--accent-cyan);
  cursor: pointer;
  box-shadow: 0 0 6px var(--accent-cyan);
}

.volume-percent {
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--text-secondary);
  min-width: 26px;
}
</style>
