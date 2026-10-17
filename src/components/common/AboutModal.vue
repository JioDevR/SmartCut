<script setup lang="ts">
defineProps<{
  isOpen: boolean;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
}>();
</script>

<template>
  <Teleport to="body">
    <div v-if="isOpen" class="about-modal-backdrop" @click.self="emit('close')">
      <div class="about-modal-card">
        <div class="about-modal-header">
          <div class="about-brand">
            <div class="about-logo-wrapper">
              <span class="material-symbols-outlined about-logo">movie_edit</span>
            </div>
            <div class="about-title-block">
              <h3 class="about-title">SmartCut Studio</h3>
              <span class="about-version">Obsidian Precision • Release 2026.10</span>
            </div>
          </div>
          <button type="button" class="btn-close-about" @click="emit('close')">✕</button>
        </div>

        <div class="about-modal-body">
          <p class="about-desc">
            Editor y reproductor de video nativo de alto rendimiento para corte quirúrgico sin pérdidas (Lossless Cut),
            Smart Rendering con aceleración por GPU y streaming local seguro por demanda.
          </p>

          <div class="about-info-grid">
            <div class="about-info-row">
              <span class="about-k">Versión de compilación:</span>
              <span class="about-v highlight">0.1.0 Beta 1 (Release Modular)</span>
            </div>
            <div class="about-info-row">
              <span class="about-k">Arquitectura:</span>
              <span class="about-v">Multiplataforma (Linux, Windows, macOS)</span>
            </div>
            <div class="about-info-row">
              <span class="about-k">Tecnologías Frontend:</span>
              <span class="about-v">Vue 3 + TypeScript + Vite + Obsidian Precision CSS</span>
            </div>
            <div class="about-info-row">
              <span class="about-k">Motor Backend:</span>
              <span class="about-v">Tauri v2 + Rust Core + FFmpeg / FFprobe</span>
            </div>
            <div class="about-info-row">
              <span class="about-k">Aceleración GPU:</span>
              <span class="about-v">NVIDIA NVENC, Intel QSV, CPU Multinúcleo</span>
            </div>
            <div class="about-info-row">
              <span class="about-k">Licencia:</span>
              <span class="about-v">Open Source</span>
            </div>
          </div>
        </div>

        <div class="about-modal-footer">
          <button type="button" class="btn-about-close" @click="emit('close')">
            Cerrar
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.about-modal-backdrop {
  position: fixed;
  inset: 0;
  background-color: rgba(5, 8, 14, 0.82);
  backdrop-filter: blur(14px);
  -webkit-backdrop-filter: blur(14px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1100;
  padding: 16px;
}

.about-modal-card {
  width: 100%;
  max-width: 500px;
  background-color: var(--surface-low);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-xl);
  overflow: hidden;
  box-shadow: 0 24px 60px rgba(0, 0, 0, 0.7);
  display: flex;
  flex-direction: column;
}

.about-modal-header {
  padding: 18px 22px;
  background-color: rgba(24, 28, 36, 0.85);
  border-bottom: 1px solid var(--border-subtle);
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.about-brand {
  display: flex;
  align-items: center;
  gap: 14px;
}

.about-logo-wrapper {
  width: 40px;
  height: 40px;
  border-radius: var(--radius-md);
  background: linear-gradient(135deg, var(--accent-cyan), var(--accent-indigo));
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 4px 12px rgba(6, 182, 212, 0.3);
}

.about-logo {
  font-size: 24px;
  color: #fff;
}

.about-title {
  font-family: var(--font-ui);
  font-size: 16px;
  font-weight: 700;
  color: var(--text-pure);
  margin: 0;
}

.about-version {
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--accent-cyan-bright);
}

.btn-close-about {
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: 18px;
  cursor: pointer;
  padding: 4px;
  border-radius: var(--radius-xs);
  transition: all 0.15s;
}

.btn-close-about:hover {
  color: var(--text-pure);
  background-color: var(--surface-high);
}

.about-modal-body {
  padding: 22px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.about-desc {
  font-family: var(--font-ui);
  font-size: 13px;
  line-height: 1.5;
  color: var(--text-secondary);
  margin: 0;
}

.about-info-grid {
  background-color: var(--surface-dim);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-lg);
  padding: 12px 16px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.about-info-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 12px;
}

.about-k {
  font-family: var(--font-ui);
  color: var(--text-muted);
}

.about-v {
  font-family: var(--font-mono);
  color: var(--text-primary);
  font-size: 11px;
}

.about-v.highlight {
  color: var(--accent-cyan);
  font-weight: 600;
}

.about-modal-footer {
  padding: 14px 22px;
  background-color: var(--surface-dim);
  border-top: 1px solid var(--border-subtle);
  display: flex;
  justify-content: flex-end;
}

.btn-about-close {
  padding: 8px 18px;
  background-color: var(--surface-high);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  font-family: var(--font-ui);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s;
}

.btn-about-close:hover {
  background-color: var(--surface-highest);
  color: var(--text-pure);
  border-color: var(--text-muted);
}
</style>
