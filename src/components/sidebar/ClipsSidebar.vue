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
}>();

const emit = defineEmits<{
  (e: 'toggle-collapse'): void;
  (e: 'select-clip', clip: ClipItem): void;
  (e: 'play-clip', clip: ClipItem): void;
  (e: 'delete-clip', id: string): void;
  (e: 'context-menu', event: MouseEvent, clip: ClipItem): void;
}>();
</script>

<template>
  <aside class="clips-sidebar" :class="{ 'is-collapsed': isCollapsed }">
    <div class="sidebar-header">
      <div class="sidebar-title-row" v-if="!isCollapsed">
        <span class="material-symbols-outlined text-cyan">video_library</span>
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
          <div class="clip-card-stripe" :style="{ background: clip.color }"></div>
          <div class="clip-card-body">
            <div class="clip-card-header">
              <span class="clip-card-name">{{ clip.name }}</span>
              <span class="clip-card-link-badge" title="Audio y Video sincronizados y vinculados">
                <span class="material-symbols-outlined">link</span>
                <span>V1+A1</span>
              </span>
            </div>

            <div class="clip-card-meta">
              <span class="clip-card-range">
                {{ formatTimecode(clip.start, metadata.fps) }} - {{ formatTimecode(clip.end, metadata.fps) }}
              </span>
              <span class="clip-card-duration">
                {{ formatTimecode(clip.end - clip.start, metadata.fps) }}
              </span>
            </div>
          </div>

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

        <div v-if="clips.length === 1" class="sidebar-hint-card">
          <span class="material-symbols-outlined text-amber">tips_and_updates</span>
          <p>Usa la <b>Cuchilla [C]</b> o haz un corte con <b>[S]</b> en la línea de tiempo para separar nuevos clips aquí.</p>
        </div>
      </div>

      <!-- Scope de Audio Master (Referencia Obsidian Precision) -->
      <div class="audio-master-scope">
        <div class="scope-header">
          <span class="scope-title">Monitoreo Master</span>
          <span class="scope-val" :class="{ 'is-active': isPlaying && ((audioLevelL ?? 0) > 0 || (audioLevelR ?? 0) > 0) }">
            {{ audioDbText || (isPlaying ? '-12.0 dBFS' : '-inf dB') }}
          </span>
        </div>
        <div class="scope-bars">
          <div class="scope-channel">
            <span class="channel-label">L</span>
            <div class="meter-track">
              <div class="meter-fill meter-cyan" :style="{ width: `${audioLevelL ?? 0}%` }"></div>
              <div class="meter-fill meter-amber" :style="{ width: `${Math.max(0, (audioLevelL ?? 0) - 80) * 5}%` }"></div>
            </div>
          </div>
          <div class="scope-channel">
            <span class="channel-label">R</span>
            <div class="meter-track">
              <div class="meter-fill meter-cyan" :style="{ width: `${audioLevelR ?? 0}%` }"></div>
              <div class="meter-fill meter-amber" :style="{ width: `${Math.max(0, (audioLevelR ?? 0) - 80) * 5}%` }"></div>
            </div>
          </div>
        </div>
      </div>

      <!-- Tarjeta de atajos de productividad -->
      <div class="shortcuts-card">
        <div class="shortcuts-card-header">
          <span class="material-symbols-outlined text-cyan">keyboard</span>
          <span>Atajos de Precisión</span>
        </div>
        <p class="shortcuts-text">
          Presiona <kbd>I</kbd> y <kbd>O</kbd> para recortar al vuelo, <kbd>S</kbd> para cortar y <kbd>Espacio</kbd> para reproducir.
        </p>
      </div>
    </div>
  </aside>
</template>

<style scoped>
.clips-sidebar {
  width: 280px;
  background-color: var(--surface-dim);
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
  height: 40px;
  min-height: 40px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 10px;
  border-bottom: 1px solid var(--border-subtle);
  background-color: rgba(19, 27, 42, 0.6);
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
  background-color: var(--accent-cyan-dim);
  color: var(--accent-cyan-bright);
  padding: 1px 6px;
  border-radius: var(--radius-xs);
}

.btn-icon-collapse {
  width: 24px;
  height: 24px;
  border-radius: var(--radius-sm);
  background: none;
  border: 1px solid transparent;
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
  border-color: var(--border-strong);
}

.sidebar-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 10px;
  overflow: hidden;
}

.clips-list-scroll {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-right: 2px;
}

/* Clip card */
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
  border-color: var(--accent-cyan);
  box-shadow: 0 0 10px rgba(6, 182, 212, 0.18);
}

.clip-card-stripe {
  width: 4px;
  align-self: stretch;
}

.clip-card-body {
  flex: 1;
  padding: 8px 10px;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.clip-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
}

.clip-card-name {
  font-family: var(--font-ui);
  font-size: 12px;
  font-weight: 500;
  color: var(--text-pure);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.clip-card-link-badge {
  display: flex;
  align-items: center;
  gap: 2px;
  font-family: var(--font-mono);
  font-size: 9px;
  color: var(--accent-cyan);
  background-color: var(--accent-cyan-dim);
  padding: 1px 4px;
  border-radius: 2px;
}

.clip-card-link-badge .material-symbols-outlined {
  font-size: 10px;
}

.clip-card-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--text-secondary);
}

.clip-card-duration {
  color: var(--accent-cyan-bright);
  font-weight: 500;
}

.clip-card-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  padding-right: 6px;
}

.clip-action-btn {
  width: 24px;
  height: 24px;
  border-radius: var(--radius-xs);
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
  background-color: var(--surface-high);
  color: var(--text-pure);
}

.btn-delete-clip:hover {
  color: var(--accent-red);
  background-color: rgba(239, 68, 68, 0.15);
}

.sidebar-hint-card {
  display: flex;
  gap: 8px;
  background-color: rgba(232, 147, 55, 0.08);
  border: 1px dashed rgba(232, 147, 55, 0.3);
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  font-size: 11px;
  color: var(--text-secondary);
  line-height: 1.4;
}

.sidebar-hint-card .material-symbols-outlined {
  font-size: 16px;
  flex-shrink: 0;
}

/* Audio Master Scope */
.audio-master-scope {
  background-color: var(--surface-low);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm);
  padding: 8px 10px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.scope-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.scope-title {
  font-family: var(--font-ui);
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-secondary);
}

.scope-val {
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--text-muted);
  transition: color 0.15s;
}

.scope-val.is-active {
  color: var(--accent-cyan);
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
  font-weight: 600;
  color: var(--text-muted);
  width: 8px;
}

.meter-track {
  flex: 1;
  height: 6px;
  background-color: var(--surface-dim);
  border-radius: 999px;
  overflow: hidden;
  display: flex;
}

.meter-fill {
  height: 100%;
  transition: width 0.1s ease-out;
}

.meter-cyan {
  background-color: var(--accent-cyan);
}

.meter-amber {
  background-color: var(--accent-amber);
}

/* Shortcuts card */
.shortcuts-card {
  background-color: var(--surface-dim);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm);
  padding: 8px 10px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.shortcuts-card-header {
  display: flex;
  align-items: center;
  gap: 6px;
  font-family: var(--font-ui);
  font-size: 11px;
  font-weight: 600;
  color: var(--text-pure);
}

.shortcuts-card-header .material-symbols-outlined {
  font-size: 14px;
}

.shortcuts-text {
  font-size: 11px;
  color: var(--text-secondary);
  line-height: 1.4;
}

kbd {
  font-family: var(--font-mono);
  font-size: 9px;
  background-color: var(--surface-high);
  border: 1px solid var(--border-strong);
  padding: 1px 4px;
  border-radius: 3px;
  color: var(--text-pure);
}
</style>
