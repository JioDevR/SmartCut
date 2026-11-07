<script setup lang="ts">
import type { ClipItem, VideoMetadata } from '../../types';
import { formatTimecode } from '../../utils/formatters';

const props = defineProps<{
  clips: ClipItem[];
  selectedClipId: string | null;
  metadata: VideoMetadata;
  isCollapsed: boolean;
  isPlaying: boolean;
  audioLevelL?: number;
  audioLevelR?: number;
  audioDbText?: string;
  volume: number;
  isMuted: boolean;
}>();

const emit = defineEmits<{
  (e: 'toggle-collapse'): void;
  (e: 'select-clip', clip: ClipItem): void;
  (e: 'play-clip', clip: ClipItem): void;
  (e: 'delete-clip', id: string): void;
  (e: 'context-menu', event: MouseEvent, clip: ClipItem): void;
  (e: 'toggle-mute'): void;
  (e: 'set-volume', val: number): void;
}>();
</script>

<template>
  <aside class="clips-sidebar" :class="{ 'is-collapsed': isCollapsed }">
    <div class="sidebar-header">
      <div class="sidebar-title-row" v-if="!isCollapsed">
        <span class="material-symbols-outlined text-green">video_library</span>
        <span class="sidebar-title">Clips & Trazas</span>
        <span class="sidebar-count-badge">{{ clips.length }}</span>
      </div>
      <button
        type="button"
        class="btn-icon-collapse"
        @click="emit('toggle-collapse')"
        :title="isCollapsed ? 'Expandir panel de clips' : 'Colapsar panel'"
      >
        <span class="material-symbols-outlined">{{ isCollapsed ? 'chevron_right' : 'chevron_left' }}</span>
      </button>
    </div>

    <!-- Contenido expandido -->
    <div v-if="!isCollapsed" class="sidebar-body">
      <!-- Lista de clips scrolleable -->
      <div class="clips-list-scroll">
        <div
          v-for="clip in clips"
          :key="'sidebar-' + clip.id"
          class="clip-card-item"
          :class="{ 'is-active': selectedClipId === clip.id }"
          @click="emit('select-clip', clip)"
          @contextmenu.prevent.stop="emit('context-menu', $event, clip)"
        >
          <div class="clip-card-body">
            <div class="clip-card-top-row">
              <span class="clip-card-name">{{ clip.name }}</span>
              <span class="clip-card-link-badge" title="Audio y Video sincronizados y vinculados">
                <span class="material-symbols-outlined">link</span>
                <span>V1+A1</span>
              </span>
            </div>

            <div class="clip-card-bottom-row">
              <span class="clip-card-range">
                {{ formatTimecode(clip.start, metadata.fps) }} - {{ formatTimecode(clip.end, metadata.fps) }}
              </span>

              <div class="clip-card-actions">
                <button
                  type="button"
                  class="clip-action-btn"
                  @click.stop="emit('play-clip', clip)"
                  title="Reproducir este clip"
                >
                  <span class="material-symbols-outlined">play_arrow</span>
                </button>
                <button
                  type="button"
                  class="clip-action-btn btn-delete-clip"
                  @click.stop="emit('delete-clip', clip.id)"
                  title="Eliminar este clip"
                >
                  <span class="material-symbols-outlined">delete</span>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Scope de Audio Master y Control de Volumen (Exacto a la captura) -->
      <div class="audio-master-scope">
        <div class="scope-header">
          <span class="scope-title">MONITOREO MASTER</span>
          <span class="scope-val" :class="{ 'is-active': isPlaying && ((audioLevelL ?? 0) > 0 || (audioLevelR ?? 0) > 0) }">
            {{ audioDbText || (isPlaying ? '-12.0 dBFS' : '-inf dBFS') }}
          </span>
        </div>
        <div class="scope-bars">
          <div class="scope-channel">
            <span class="channel-label">L</span>
            <div class="meter-track">
              <div class="meter-fill meter-primary" :style="{ width: `${audioLevelL ?? 0}%` }"></div>
              <div class="meter-fill meter-peak" :style="{ width: `${Math.max(0, (audioLevelL ?? 0) - 80) * 5}%` }"></div>
            </div>
          </div>
          <div class="scope-channel">
            <span class="channel-label">R</span>
            <div class="meter-track">
              <div class="meter-fill meter-primary" :style="{ width: `${audioLevelR ?? 0}%` }"></div>
              <div class="meter-fill meter-peak" :style="{ width: `${Math.max(0, (audioLevelR ?? 0) - 80) * 5}%` }"></div>
            </div>
          </div>
        </div>

        <!-- Control de Volumen Master integrado aquí -->
        <div class="sidebar-volume-row">
          <button
            type="button"
            class="sidebar-vol-btn"
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
            class="sidebar-vol-slider"
            :value="isMuted ? 0 : volume"
            @input="emit('set-volume', Number(($event.target as HTMLInputElement).value))"
          />
          <span class="sidebar-vol-pct">{{ Math.round((isMuted ? 0 : volume) * 100) }}%</span>
        </div>
      </div>
    </div>
  </aside>
</template>

<style scoped>
.clips-sidebar {
  width: 260px;
  background-color: #0b0f17;
  border-right: 1px solid var(--border-subtle);
  display: flex;
  flex-direction: column;
  transition: width 0.2s cubic-bezier(0.2, 0, 0, 1);
  overflow: hidden;
  user-select: none;
  flex-shrink: 0;
}

.clips-sidebar.is-collapsed {
  width: 42px;
}

.sidebar-header {
  height: 42px;
  min-height: 42px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px;
  border-bottom: 1px solid var(--border-subtle);
  background-color: rgba(14, 18, 26, 0.95);
}

.sidebar-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.sidebar-title {
  font-family: var(--font-ui);
  font-size: 12px;
  font-weight: 600;
  color: var(--text-pure);
  letter-spacing: -0.01em;
}

.sidebar-count-badge {
  font-family: var(--font-mono);
  font-size: 10px;
  font-weight: 600;
  background-color: rgba(16, 185, 129, 0.12);
  color: #10b981;
  padding: 1px 5px;
  border-radius: var(--radius-xs);
}

.btn-icon-collapse {
  width: 24px;
  height: 24px;
  border-radius: var(--radius-sm);
  background: none;
  border: none;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.15s;
}

.btn-icon-collapse:hover {
  background-color: var(--surface-high);
  color: var(--text-pure);
}

.sidebar-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  padding: 10px;
  overflow: hidden;
}

.clips-list-scroll {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding-right: 2px;
}

/* Tarjeta de clip compacta */
.clip-card-item {
  display: flex;
  align-items: center;
  background-color: var(--surface-low);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm);
  overflow: hidden;
  transition: all 0.15s;
  cursor: pointer;
}

.clip-card-item:hover {
  background-color: var(--surface-mid);
  border-color: var(--border-strong);
}

.clip-card-item.is-active {
  background-color: var(--surface-mid);
  border-color: #10b981;
  box-shadow: 0 0 10px rgba(16, 185, 129, 0.15);
}

.clip-card-body {
  flex: 1;
  padding: 8px 10px;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.clip-card-top-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.clip-card-name {
  font-family: var(--font-ui);
  font-size: 12px;
  font-weight: 500;
  color: var(--text-pure);
}

.clip-card-link-badge {
  display: flex;
  align-items: center;
  gap: 2px;
  font-family: var(--font-mono);
  font-size: 9px;
  color: var(--text-secondary);
  background-color: rgba(255, 255, 255, 0.06);
  padding: 1px 4px;
  border-radius: 2px;
}

.clip-card-link-badge .material-symbols-outlined {
  font-size: 10px;
}

.clip-card-bottom-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.clip-card-range {
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--text-secondary);
}

.clip-card-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}

.clip-action-btn {
  width: 22px;
  height: 22px;
  border-radius: 3px;
  background: none;
  border: none;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.15s;
}

.clip-action-btn:hover {
  background-color: rgba(255, 255, 255, 0.1);
  color: var(--text-pure);
}

.clip-action-btn .material-symbols-outlined {
  font-size: 14px;
}

.btn-delete-clip:hover {
  color: #f87171;
  background-color: rgba(239, 68, 68, 0.15);
}

/* Monitoreo Master */
.audio-master-scope {
  background-color: rgba(14, 18, 26, 0.7);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm);
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 8px;
}

.scope-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.scope-title {
  font-family: var(--font-mono);
  font-size: 9px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-secondary);
}

.scope-val {
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--text-muted);
}

.scope-val.is-active {
  color: #10b981;
}

.scope-bars {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.scope-channel {
  display: flex;
  align-items: center;
  gap: 6px;
}

.channel-label {
  font-family: var(--font-mono);
  font-size: 9px;
  color: var(--text-muted);
  width: 10px;
}

.meter-track {
  flex: 1;
  height: 4px;
  background-color: rgba(255, 255, 255, 0.06);
  border-radius: 2px;
  overflow: hidden;
  position: relative;
}

.meter-fill {
  position: absolute;
  top: 0;
  left: 0;
  bottom: 0;
  transition: width 0.05s ease-out;
}

.meter-primary {
  background-color: var(--accent-primary);
  box-shadow: 0 0 6px var(--accent-primary-glow);
}

.meter-peak {
  background-color: var(--accent-primary-bright);
}

/* Control de volumen integrado */
.sidebar-volume-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding-top: 4px;
  border-top: 1px solid rgba(255, 255, 255, 0.05);
}

.sidebar-vol-btn {
  background: none;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
}

.sidebar-vol-btn:hover {
  color: var(--text-pure);
}

.sidebar-vol-btn .material-symbols-outlined {
  font-size: 16px;
}

.sidebar-vol-slider {
  flex: 1;
  height: 4px;
  -webkit-appearance: none;
  appearance: none;
  background: rgba(255, 255, 255, 0.12);
  border-radius: 2px;
  outline: none;
}

.sidebar-vol-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: #10b981;
  cursor: pointer;
  box-shadow: 0 0 4px rgba(16, 185, 129, 0.6);
}

.sidebar-vol-pct {
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--text-secondary);
  width: 28px;
  text-align: right;
}
</style>
