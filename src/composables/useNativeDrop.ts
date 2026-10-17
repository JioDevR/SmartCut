import { ref, onMounted, onUnmounted } from 'vue';
import { getCurrentWebview } from '@tauri-apps/api/webview';

export interface NativeDropOptions {
  onFileDrop: (filePath: string) => void;
}

export function useNativeDrop(options: NativeDropOptions) {
  const isDraggingFile = ref(false);
  let unlistenDrop: (() => void) | null = null;

  function onHtmlDragOver(e: DragEvent) {
    e.preventDefault();
    isDraggingFile.value = true;
  }

  function onHtmlDragLeave(e: DragEvent) {
    if (e.relatedTarget === null || (e.currentTarget as HTMLElement)?.contains?.(e.relatedTarget as Node)) {
      return;
    }
    isDraggingFile.value = false;
  }

  function onHtmlDrop(e: DragEvent) {
    e.preventDefault();
    isDraggingFile.value = false;
    if (e.dataTransfer && e.dataTransfer.files.length > 0) {
      const file = e.dataTransfer.files[0];
      const path = (file as any).path;
      if (path) {
        options.onFileDrop(path);
      }
    }
  }

  onMounted(async () => {
    try {
      const webview = getCurrentWebview();
      unlistenDrop = await webview.onDragDropEvent((event) => {
        if (event.payload.type === 'over') {
          isDraggingFile.value = true;
        } else if (event.payload.type === 'drop') {
          isDraggingFile.value = false;
          const paths = event.payload.paths;
          if (paths && paths.length > 0) {
            options.onFileDrop(paths[0]);
          }
        } else {
          isDraggingFile.value = false;
        }
      });
    } catch (err) {
      console.warn('Tauri onDragDropEvent fallback to HTML5 drop:', err);
    }
  });

  onUnmounted(() => {
    if (unlistenDrop) {
      unlistenDrop();
      unlistenDrop = null;
    }
  });

  return {
    isDraggingFile,
    onHtmlDragOver,
    onHtmlDragLeave,
    onHtmlDrop,
  };
}
