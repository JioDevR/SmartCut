import { onMounted, onUnmounted } from 'vue';
import type { ActiveTool } from '../types';

export interface ShortcutActions {
  onUndo: () => void;
  onRedo: () => void;
  onOpenFile: () => void;
  onOpenExport: () => void;
  onSaveProject: () => void;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onStepFrame: (delta: number) => void;
  onStepSeconds: (delta: number) => void;
  onTogglePlay: () => void;
  onSetTool: (tool: ActiveTool) => void;
  onDeleteSelectedClip: () => void;
  onMarkIn: () => void;
  onMarkOut: () => void;
  onSplitAtPlayhead: () => void;
  onSplitByInOut: () => void;
  onToggleFastPlayback: () => void;
  onPauseIfPlaying: () => void;
  onCloseContextMenu: () => void;
  isContextMenuVisible: () => boolean;
  hasSelectedClip: () => boolean;
}

export function useKeyboardShortcuts(actions: ShortcutActions) {
  function handleKeydown(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;

    const isMod = e.ctrlKey || e.metaKey;

    // Atajos con Ctrl / Cmd
    if (isMod) {
      switch (e.code) {
        case 'KeyZ':
          e.preventDefault();
          if (e.shiftKey) {
            actions.onRedo();
          } else {
            actions.onUndo();
          }
          break;
        case 'KeyY':
          e.preventDefault();
          actions.onRedo();
          break;
        case 'KeyO':
          e.preventDefault();
          actions.onOpenFile();
          break;
        case 'KeyE':
          e.preventDefault();
          actions.onOpenExport();
          break;
        case 'KeyS':
          e.preventDefault();
          actions.onSaveProject();
          break;
        case 'Equal':
        case 'NumpadAdd':
          e.preventDefault();
          actions.onZoomIn();
          break;
        case 'Minus':
        case 'NumpadSubtract':
          e.preventDefault();
          actions.onZoomOut();
          break;
        case 'ArrowLeft':
          e.preventDefault();
          actions.onStepFrame(-1);
          break;
        case 'ArrowRight':
          e.preventDefault();
          actions.onStepFrame(1);
          break;
      }
      return;
    }

    // Atajos con tecla Alt
    if (e.altKey) {
      if (e.code === 'ArrowLeft') {
        e.preventDefault();
        actions.onStepFrame(-1);
      } else if (e.code === 'ArrowRight') {
        e.preventDefault();
        actions.onStepFrame(1);
      }
      return;
    }

    // Atajos simples (sin Ctrl / Cmd / Alt)
    switch (e.code) {
      case 'Escape':
        if (actions.isContextMenuVisible()) {
          e.preventDefault();
          actions.onCloseContextMenu();
        }
        break;
      case 'KeyV':
        e.preventDefault();
        actions.onSetTool('select');
        break;
      case 'KeyC':
        e.preventDefault();
        actions.onSetTool('cut');
        break;
      case 'Space':
        e.preventDefault();
        actions.onTogglePlay();
        break;
      case 'ArrowLeft':
        e.preventDefault();
        if (e.shiftKey) actions.onStepSeconds(-5);
        else actions.onStepSeconds(-1);
        break;
      case 'ArrowRight':
        e.preventDefault();
        if (e.shiftKey) actions.onStepSeconds(5);
        else actions.onStepSeconds(1);
        break;
      case 'Delete':
      case 'Backspace':
        if (actions.hasSelectedClip()) {
          e.preventDefault();
          actions.onDeleteSelectedClip();
        }
        break;
      case 'KeyI':
        e.preventDefault();
        actions.onMarkIn();
        break;
      case 'KeyO':
        e.preventDefault();
        actions.onMarkOut();
        break;
      case 'KeyS':
        e.preventDefault();
        actions.onSplitAtPlayhead();
        break;
      case 'KeyX':
        e.preventDefault();
        actions.onSplitByInOut();
        break;
      case 'KeyL':
        e.preventDefault();
        actions.onToggleFastPlayback();
        break;
      case 'KeyK':
        e.preventDefault();
        actions.onPauseIfPlaying();
        break;
      case 'KeyJ':
        e.preventDefault();
        actions.onStepSeconds(-2);
        break;
    }
  }

  onMounted(() => {
    window.addEventListener('keydown', handleKeydown);
  });

  onUnmounted(() => {
    window.removeEventListener('keydown', handleKeydown);
  });

  return {
    handleKeydown,
  };
}
