<script setup lang="ts">
const emit = defineEmits<{
  (e: 'open-file'): void;
}>();
</script>

<template>
  <div class="empty-stage">
    <div class="empty-card">
      <div class="empty-icon-ring">
        <span class="material-symbols-outlined">content_cut</span>
      </div>
      <h2 class="empty-title">SmartCut</h2>
      <p class="empty-desc">
        Corte y edición de video sin pérdida y al fotograma exacto.
      </p>

      <div class="empty-actions">
        <button type="button" class="btn-primary-large" @click="emit('open-file')">
          <span class="material-symbols-outlined">folder_open</span>
          <span>Abrir Archivo de Video</span>
          <span class="empty-shortcut">⌘O</span>
        </button>
      </div>

      <p class="empty-drag-hint">o arrastra y suelta tu archivo aquí</p>
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
  max-width: 500px;
  background-color: var(--surface-low);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-xl);
  padding: 36px 40px;
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
  background: rgba(16, 185, 129, 0.1);
  border: 1px solid rgba(16, 185, 129, 0.25);
  display: flex;
  align-items: center;
  justify-content: center;
  color: #10b981;
  margin-bottom: 16px;
}

.empty-icon-ring .material-symbols-outlined {
  font-size: 28px;
}

.empty-title {
  font-family: var(--font-ui);
  font-size: 22px;
  font-weight: 600;
  color: var(--text-pure);
  margin-bottom: 8px;
}

.empty-desc {
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.5;
  margin-bottom: 24px;
}

.empty-actions {
  display: flex;
  gap: 12px;
  margin-bottom: 12px;
}

.btn-primary-large {
  display: flex;
  align-items: center;
  gap: 8px;
  background-color: #ffffff;
  color: #090d16;
  border: none;
  padding: 10px 20px;
  border-radius: var(--radius-md);
  font-family: var(--font-ui);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
}

.btn-primary-large:hover {
  background-color: #f1f5f9;
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(255, 255, 255, 0.2);
}

.empty-shortcut {
  font-family: var(--font-mono);
  font-size: 11px;
  background-color: rgba(0, 0, 0, 0.08);
  padding: 2px 5px;
  border-radius: 4px;
  color: #1e293b;
}

.empty-drag-hint {
  font-size: 11px;
  color: var(--text-muted);
}
</style>
