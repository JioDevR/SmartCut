<script setup lang="ts">
import type { SampleVideoInfo } from '../../types';

defineProps<{
  sampleVideos: SampleVideoInfo[];
}>();

const emit = defineEmits<{
  (e: 'open-file'): void;
  (e: 'load-sample', path: string): void;
}>();
</script>

<template>
  <div class="empty-stage">
    <div class="empty-card">
      <div class="empty-icon-ring">
        <span class="material-symbols-outlined">movie_filter</span>
      </div>
      <h2 class="empty-title">SmartCut Obsidian Studio</h2>
      <p class="empty-desc">
        Edición y recorte de video de alta precisión sin pérdida con streaming de rangos HTTP 206 y Smart Rendering por GPU.
      </p>

      <div class="empty-actions">
        <button type="button" class="btn-primary-large" @click="emit('open-file')">
          <span class="material-symbols-outlined">folder_open</span>
          <span>Abrir Archivo de Video</span>
          <span class="empty-shortcut">⌘O</span>
        </button>
      </div>

      <!-- Muestras rápidas detectadas -->
      <div class="sample-cards" v-if="sampleVideos.length > 0">
        <span class="sample-cards-title">Archivos de prueba disponibles:</span>
        <div class="sample-grid">
          <div
            v-for="sample in sampleVideos"
            :key="sample.path"
            class="sample-card"
            @click="emit('load-sample', sample.path)"
          >
            <div class="sample-card-header">
              <span class="sample-chip">{{ sample.format }}</span>
              <span class="sample-size">{{ sample.size_formatted }}</span>
            </div>
            <div class="sample-card-title">{{ sample.name }}</div>
            <div class="sample-card-action">Cargar en el editor →</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.empty-stage {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
}

.empty-card {
  max-width: 580px;
  background-color: var(--surface-low);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-xl);
  padding: 32px 36px;
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  box-shadow: 0 16px 40px rgba(0, 0, 0, 0.45);
}

.empty-icon-ring {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  background: linear-gradient(135deg, rgba(6, 182, 212, 0.2), rgba(99, 102, 241, 0.2));
  border: 1px solid rgba(6, 182, 212, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--accent-cyan);
  margin-bottom: 16px;
}

.empty-icon-ring .material-symbols-outlined {
  font-size: 28px;
}

.empty-title {
  font-family: var(--font-ui);
  font-size: 20px;
  font-weight: 600;
  color: var(--text-pure);
  letter-spacing: -0.015em;
  margin-bottom: 8px;
}

.empty-desc {
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.5;
  margin-bottom: 22px;
}

.empty-actions {
  display: flex;
  gap: 12px;
  margin-bottom: 24px;
}

.btn-primary-large {
  display: flex;
  align-items: center;
  gap: 10px;
  height: 42px;
  padding: 0 22px;
  border-radius: var(--radius-sm);
  background: linear-gradient(135deg, #06b6d4, #0284c7);
  border: 1px solid rgba(76, 215, 246, 0.5);
  color: #04131f;
  font-family: var(--font-ui);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s;
  box-shadow: 0 0 20px rgba(6, 182, 212, 0.4);
}

.btn-primary-large:hover {
  filter: brightness(1.1);
  transform: translateY(-1px);
}

.empty-shortcut {
  font-family: var(--font-mono);
  font-size: 11px;
  background-color: rgba(4, 19, 31, 0.25);
  padding: 2px 5px;
  border-radius: 3px;
}

.sample-cards {
  width: 100%;
  border-top: 1px solid var(--border-subtle);
  padding-top: 18px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.sample-cards-title {
  font-size: 11px;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.04em;
  font-weight: 600;
}

.sample-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 10px;
}

.sample-card {
  background-color: var(--surface-dim);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-md);
  padding: 10px 12px;
  text-align: left;
  cursor: pointer;
  transition: all 0.15s;
}

.sample-card:hover {
  background-color: var(--surface-high);
  border-color: var(--accent-cyan);
  transform: translateY(-1px);
}

.sample-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
}

.sample-chip {
  font-family: var(--font-mono);
  font-size: 9px;
  font-weight: 600;
  background-color: var(--surface-highest);
  color: var(--accent-cyan);
  padding: 1px 4px;
  border-radius: 2px;
}

.sample-size {
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--text-muted);
}

.sample-card-title {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-pure);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-bottom: 4px;
}

.sample-card-action {
  font-size: 11px;
  color: var(--accent-cyan);
}
</style>
