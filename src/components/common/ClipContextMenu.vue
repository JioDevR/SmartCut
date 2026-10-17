<script setup lang="ts">
import type { ClipItem } from '../../types';
import { formatTimecode } from '../../utils/formatters';

defineProps<{
  visible: boolean;
  x: number;
  y: number;
  clip: ClipItem | null;
  fps: number;
}>();

const emit = defineEmits<{
  (e: 'play'): void;
  (e: 'set-in-out'): void;
  (e: 'close-gap-before'): void;
  (e: 'close-all-gaps'): void;
  (e: 'keep-only'): void;
  (e: 'delete'): void;
  (e: 'close'): void;
}>();
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible && clip"
      class="clip-context-menu"
      :style="{ top: `${y}px`, left: `${x}px` }"
      @click.stop
    >
      <div class="ctx-header">
        <span class="ctx-clip-dot" :style="{ background: clip.color }"></span>
        <span class="ctx-clip-title">{{ clip.name }}</span>
        <span class="ctx-clip-dur">{{ formatTimecode(clip.end - clip.start, fps) }}</span>
      </div>

      <div class="ctx-divider"></div>

      <button type="button" class="ctx-item" @click="emit('play')">
        <span class="material-symbols-outlined">play_arrow</span>
        <span>Reproducir este clip</span>
      </button>

      <button type="button" class="ctx-item" @click="emit('set-in-out')">
        <span class="material-symbols-outlined text-cyan">first_page</span>
        <span>Fijar puntos In/Out</span>
      </button>

      <button type="button" class="ctx-item" @click="emit('close-gap-before')">
        <span class="material-symbols-outlined text-emerald">fast_rewind</span>
        <span>Cerrar hueco anterior</span>
      </button>

      <button type="button" class="ctx-item" @click="emit('close-all-gaps')">
        <span class="material-symbols-outlined text-amber">compress</span>
        <span>Cerrar todos los huecos</span>
      </button>

      <button type="button" class="ctx-item" @click="emit('keep-only')">
        <span class="material-symbols-outlined text-sky">filter_alt</span>
        <span>Mantener solo este clip</span>
      </button>

      <div class="ctx-divider"></div>

      <button type="button" class="ctx-item ctx-item-danger" @click="emit('delete')">
        <span class="material-symbols-outlined text-rose">delete</span>
        <span>Eliminar clip</span>
        <span class="ctx-shortcut">Supr</span>
      </button>
    </div>
  </Teleport>
</template>

<style scoped>
.clip-context-menu {
  position: fixed;
  width: 220px;
  background-color: rgba(19, 27, 42, 0.95);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-md);
  padding: 5px;
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.65);
  z-index: 2000;
  display: flex;
  flex-direction: column;
  user-select: none;
}

.ctx-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
}

.ctx-clip-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.ctx-clip-title {
  font-family: var(--font-ui);
  font-size: 11px;
  font-weight: 600;
  color: var(--text-pure);
  flex: 1;
}

.ctx-clip-dur {
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--accent-cyan);
}

.ctx-divider {
  height: 1px;
  background-color: var(--border-subtle);
  margin: 4px 0;
}

.ctx-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  background: none;
  border: none;
  border-radius: var(--radius-xs);
  color: var(--text-primary);
  font-family: var(--font-ui);
  font-size: 11px;
  cursor: pointer;
  text-align: left;
  transition: all 0.12s;
}

.ctx-item .material-symbols-outlined {
  font-size: 15px;
}

.ctx-item:hover {
  background-color: var(--surface-high);
  color: var(--text-pure);
}

.ctx-item-danger:hover {
  background-color: rgba(239, 68, 68, 0.15);
  color: var(--accent-red);
}

.text-rose {
  color: var(--accent-red);
}

.ctx-shortcut {
  margin-left: auto;
  font-family: var(--font-mono);
  font-size: 9px;
  color: var(--text-muted);
}
</style>
