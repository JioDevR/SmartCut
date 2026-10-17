import { ref, type Ref } from 'vue';
import type { ClipItem } from '../types';

export interface UseClipDragOptions {
  clips: Ref<ClipItem[]>;
  duration: Ref<number>;
  pixelsPerSecond: Ref<number>;
  activeTool: Ref<'select' | 'cut'>;
  timelineTrack: Ref<HTMLElement | null>;
  selectClip: (id: string) => void;
  handleCutAtTime: (t: number) => void;
  handleSeek: (t: number) => void;
  fastSeekTo: (t: number) => void;
  recordSnapshot: () => void;
  renumberClips: () => void;
  syncSplitPointsFromClips: () => void;
}

export function useClipDrag(options: UseClipDragOptions) {
  const {
    clips,
    duration,
    pixelsPerSecond,
    activeTool,
    timelineTrack,
    selectClip,
    handleCutAtTime,
    handleSeek,
    fastSeekTo,
    recordSnapshot,
    renumberClips,
    syncSplitPointsFromClips,
  } = options;

  const isDraggingClip = ref(false);
  const draggingClipId = ref<string | null>(null);

  let clipDragStartX = 0;
  let clipOrigStart = 0;
  let clipOrigEnd = 0;
  let clipMinStart = 0;
  let clipMaxStart = 0;
  let clipHasMoved = false;
  let pendingClipMoveRaf: number | null = null;

  function onClipPointerDown(e: PointerEvent, clip: ClipItem) {
    if (e.button !== 0) return;

    if (activeTool.value === 'cut') {
      e.stopPropagation();
      e.preventDefault();
      if (timelineTrack.value) {
        const rect = timelineTrack.value.getBoundingClientRect();
        const clickX = Math.max(0, Math.min(e.clientX - rect.left, rect.width));
        handleCutAtTime(clickX / pixelsPerSecond.value);
      }
      return;
    }

    selectClip(clip.id);
    if (timelineTrack.value) {
      const rect = timelineTrack.value.getBoundingClientRect();
      const clickX = Math.max(0, Math.min(e.clientX - rect.left, rect.width));
      handleSeek(clickX / pixelsPerSecond.value);
    }

    const dur = clip.end - clip.start;
    clipDragStartX = e.clientX;
    clipOrigStart = clip.start;
    clipOrigEnd = clip.end;
    clipHasMoved = false;

    const sorted = [...clips.value].sort((a, b) => a.start - b.start);
    const curIdx = sorted.findIndex((c) => c.id === clip.id);
    const prev = curIdx > 0 ? sorted[curIdx - 1] : null;
    const next = curIdx < sorted.length - 1 ? sorted[curIdx + 1] : null;

    clipMinStart = prev ? prev.end : 0;
    const rawMaxStart = next ? next.start - dur : duration.value - dur;
    clipMaxStart = Math.max(clipMinStart, rawMaxStart);
    draggingClipId.value = clip.id;

    try {
      (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    } catch (_) {}
  }

  function onClipPointerMove(e: PointerEvent, clip: ClipItem) {
    if (draggingClipId.value !== clip.id) return;
    const deltaPx = e.clientX - clipDragStartX;

    if (!clipHasMoved && Math.abs(deltaPx) > 4) {
      recordSnapshot();
      clipHasMoved = true;
      isDraggingClip.value = true;
    }

    if (clipHasMoved) {
      const deltaSec = deltaPx / pixelsPerSecond.value;
      const dur = clipOrigEnd - clipOrigStart;
      let newStart = clipOrigStart + deltaSec;

      const minBound = Math.min(clipMinStart, clipMaxStart);
      const maxBound = Math.max(clipMinStart, clipMaxStart);

      // Snapping a los bordes
      const snapSec = 14 / pixelsPerSecond.value;
      if (minBound >= maxBound) {
        newStart = minBound;
      } else if (Math.abs(newStart - minBound) < snapSec) {
        newStart = minBound;
      } else if (Math.abs(newStart - maxBound) < snapSec) {
        newStart = maxBound;
      } else {
        newStart = Math.max(minBound, Math.min(newStart, maxBound));
      }

      clip.start = newStart;
      clip.end = newStart + dur;

      if (!pendingClipMoveRaf) {
        pendingClipMoveRaf = requestAnimationFrame(() => {
          pendingClipMoveRaf = null;
          fastSeekTo(clip.start);
        });
      }
    }
  }

  function onClipPointerUp(e: PointerEvent, clip: ClipItem) {
    if (draggingClipId.value === clip.id) {
      if (pendingClipMoveRaf) {
        cancelAnimationFrame(pendingClipMoveRaf);
        pendingClipMoveRaf = null;
      }
      try {
        (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
      } catch (_) {}
      draggingClipId.value = null;

      if (clipHasMoved) {
        renumberClips();
        syncSplitPointsFromClips();
        handleSeek(clip.start);
      }
      isDraggingClip.value = false;
      clipHasMoved = false;
    }
  }

  // Trimming interactivo de extremos de clips (Handles)
  function onTrimHandleDown({ event, edge, clip }: { event: PointerEvent; edge: 'left' | 'right'; clip: ClipItem }) {
    event.stopPropagation();
    event.preventDefault();

    recordSnapshot();
    selectClip(clip.id);

    const startX = event.clientX;
    const origStart = clip.start;
    const origEnd = clip.end;
    const origSrcStart = clip.sourceStart !== undefined ? clip.sourceStart : clip.start;
    const origSrcEnd = clip.sourceEnd !== undefined ? clip.sourceEnd : clip.end;

    const sorted = [...clips.value].sort((a, b) => a.start - b.start);
    const curIdx = sorted.findIndex((c) => c.id === clip.id);
    const prev = curIdx > 0 ? sorted[curIdx - 1] : null;
    const next = curIdx < sorted.length - 1 ? sorted[curIdx + 1] : null;

    const minStart = prev ? prev.end : 0;
    const maxStart = origEnd - 0.2;

    const minEnd = origStart + 0.2;
    const maxEnd = next ? next.start : duration.value;

    let pendingTrimRaf: number | null = null;
    let latestTrimSeek: number | null = null;

    const handlePointerMove = (e: PointerEvent) => {
      const deltaPx = e.clientX - startX;
      const deltaSec = deltaPx / pixelsPerSecond.value;

      if (edge === 'left') {
        const newStart = Math.max(minStart, Math.min(origStart + deltaSec, maxStart));
        const diff = newStart - origStart;
        clip.start = newStart;
        clip.sourceStart = Math.max(0, origSrcStart + diff);
        latestTrimSeek = newStart;
      } else {
        const newEnd = Math.max(minEnd, Math.min(origEnd + deltaSec, maxEnd));
        const diff = newEnd - origEnd;
        clip.end = newEnd;
        clip.sourceEnd = Math.max((clip.sourceStart ?? 0) + 0.2, origSrcEnd + diff);
        latestTrimSeek = newEnd;
      }

      if (!pendingTrimRaf) {
        pendingTrimRaf = requestAnimationFrame(() => {
          pendingTrimRaf = null;
          if (latestTrimSeek !== null) {
            fastSeekTo(latestTrimSeek);
          }
        });
      }
    };

    const handlePointerUp = () => {
      if (pendingTrimRaf) {
        cancelAnimationFrame(pendingTrimRaf);
        pendingTrimRaf = null;
      }
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', handlePointerUp);
      window.removeEventListener('pointercancel', handlePointerUp);

      renumberClips();
      syncSplitPointsFromClips();
      handleSeek(edge === 'left' ? clip.start : clip.end);
    };

    window.addEventListener('pointermove', handlePointerMove);
    window.addEventListener('pointerup', handlePointerUp);
    window.addEventListener('pointercancel', handlePointerUp);
  }

  return {
    isDraggingClip,
    draggingClipId,
    onClipPointerDown,
    onClipPointerMove,
    onClipPointerUp,
    onTrimHandleDown,
  };
}
