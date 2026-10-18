import { ref, computed } from 'vue';
import type { ClipItem, TimelineGap, ActiveTool } from '../types';

export const CLIP_COLORS = [
  '#06b6d4', // Cyan (Obsidian Primary)
  '#6366f1', // Indigo (Secondary)
  '#ffb873', // Amber / Warm Orange (Tertiary)
  '#10b981', // Emerald
  '#ec4899', // Pink
  '#a855f7', // Purple
  '#38bdf8', // Sky Blue
  '#f97316', // Orange
];

export function useClipsManager() {
  const clips = ref<ClipItem[]>([]);
  const selectedClipId = ref<string | null>(null);
  const splitPoints = ref<number[]>([]);
  const inPoint = ref(0);
  const outPoint = ref(0);
  const activeTool = ref<ActiveTool>('select');
  const isSidebarCollapsed = ref(false);

  const selectedClip = computed(() => {
    return clips.value.find((c) => c.id === selectedClipId.value) || null;
  });

  const sortedClips = computed(() => {
    return [...clips.value].sort((a, b) => a.start - b.start);
  });

  const timelineGaps = computed<TimelineGap[]>(() => {
    const gaps: TimelineGap[] = [];
    const sorted = sortedClips.value;
    if (sorted.length === 0) return gaps;

    if (sorted[0].start > 0.05) {
      gaps.push({ start: 0, end: sorted[0].start });
    }
    for (let i = 0; i < sorted.length - 1; i++) {
      const curEnd = sorted[i].end;
      const nextStart = sorted[i + 1].start;
      if (nextStart - curEnd > 0.05) {
        gaps.push({ start: curEnd, end: nextStart });
      }
    }
    return gaps;
  });

  const undoStack = ref<Array<{
    clips: ClipItem[];
    selectedClipId: string | null;
    splitPoints: number[];
    inPoint: number;
    outPoint: number;
  }>>([]);
  const redoStack = ref<Array<{
    clips: ClipItem[];
    selectedClipId: string | null;
    splitPoints: number[];
    inPoint: number;
    outPoint: number;
  }>>([]);
  const MAX_HISTORY = 40;

  function createSnapshot() {
    return {
      clips: JSON.parse(JSON.stringify(clips.value)),
      selectedClipId: selectedClipId.value,
      splitPoints: [...splitPoints.value],
      inPoint: inPoint.value,
      outPoint: outPoint.value,
    };
  }

  function recordSnapshot() {
    if (clips.value.length === 0) return;
    undoStack.value.push(createSnapshot());
    if (undoStack.value.length > MAX_HISTORY) {
      undoStack.value.shift();
    }
    redoStack.value = [];
  }

  const canUndo = computed(() => undoStack.value.length > 0);
  const canRedo = computed(() => redoStack.value.length > 0);

  function undo(seekFn?: (time: number) => void) {
    if (undoStack.value.length === 0) return;
    const current = createSnapshot();
    redoStack.value.push(current);

    const prev = undoStack.value.pop()!;
    clips.value = prev.clips;
    selectedClipId.value = prev.selectedClipId;
    splitPoints.value = prev.splitPoints;
    inPoint.value = prev.inPoint;
    outPoint.value = prev.outPoint;

    if (selectedClip.value) {
      seekFn?.(selectedClip.value.start);
    }
  }

  function redo(seekFn?: (time: number) => void) {
    if (redoStack.value.length === 0) return;
    const current = createSnapshot();
    undoStack.value.push(current);

    const next = redoStack.value.pop()!;
    clips.value = next.clips;
    selectedClipId.value = next.selectedClipId;
    splitPoints.value = next.splitPoints;
    inPoint.value = next.inPoint;
    outPoint.value = next.outPoint;

    if (selectedClip.value) {
      seekFn?.(selectedClip.value.start);
    }
  }

  function initClips(dur: number) {
    if (dur <= 0) return;
    undoStack.value = [];
    redoStack.value = [];
    clips.value = [
      {
        id: 'clip-1',
        name: 'Clip 1',
        start: 0,
        end: dur,
        sourceStart: 0,
        sourceEnd: dur,
        color: CLIP_COLORS[0],
      },
    ];
    selectedClipId.value = 'clip-1';
    inPoint.value = 0;
    outPoint.value = dur;
    syncSplitPointsFromClips();
  }

  function syncSplitPointsFromClips() {
    const points: number[] = [];
    const sorted = [...clips.value].sort((a, b) => a.start - b.start);
    for (let i = 0; i < sorted.length; i++) {
      if (sorted[i].start > 0.05 && !points.includes(sorted[i].start)) {
        points.push(sorted[i].start);
      }
    }
    splitPoints.value = points.sort((a, b) => a - b);
  }

  function renumberClips() {
    const sorted = [...clips.value].sort((a, b) => a.start - b.start);
    sorted.forEach((c, idx) => {
      c.name = `Clip ${idx + 1}`;
      c.color = CLIP_COLORS[idx % CLIP_COLORS.length];
    });
    clips.value = sorted;
  }

  function selectClip(id: string) {
    selectedClipId.value = id;
    const clip = clips.value.find((c) => c.id === id);
    if (clip) {
      inPoint.value = clip.start;
      outPoint.value = clip.end;
    }
  }

  function splitAtTime(t: number, totalDuration: number, seekFn?: (time: number) => void) {
    if (clips.value.length === 0) {
      if (totalDuration > 0) initClips(totalDuration);
      else return;
    }

    // VF-13: Asegurar que clips esté ordenado cronológicamente antes del corte
    clips.value.sort((a, b) => a.start - b.start);

    const index = clips.value.findIndex((c) => t > c.start + 0.05 && t < c.end - 0.05);
    if (index === -1) return;

    recordSnapshot();

    const targetClip = clips.value[index];
    const originalEnd = targetClip.end;
    const originalSourceEnd = targetClip.sourceEnd !== undefined ? targetClip.sourceEnd : originalEnd;
    const originalSourceStart = targetClip.sourceStart !== undefined ? targetClip.sourceStart : targetClip.start;

    const splitOffset = t - targetClip.start;
    const sourceSplit = originalSourceStart + splitOffset;

    targetClip.end = t;
    targetClip.sourceEnd = sourceSplit;

    const newClipId = 'clip-' + Math.random().toString(36).substring(2, 8);
    const newClip: ClipItem = {
      id: newClipId,
      name: `Clip ${clips.value.length + 1}`,
      start: t,
      end: originalEnd,
      sourceStart: sourceSplit,
      sourceEnd: originalSourceEnd,
      color: CLIP_COLORS[(index + 1) % CLIP_COLORS.length],
    };

    clips.value.splice(index + 1, 0, newClip);
    renumberClips();
    selectedClipId.value = newClipId;
    syncSplitPointsFromClips();
    seekFn?.(t);
  }

  function splitAtCurrentPlayhead(currentTime: number, totalDuration: number, seekFn?: (time: number) => void) {
    splitAtTime(currentTime, totalDuration, seekFn);
  }

  function splitByInOut(totalDuration: number, seekFn?: (time: number) => void) {
    const inVal = inPoint.value;
    const outVal = outPoint.value > 0 ? outPoint.value : totalDuration;
    if (outVal <= inVal + 0.05) return;
    recordSnapshot();
    splitAtTime(inVal, totalDuration, seekFn);
    splitAtTime(outVal, totalDuration, seekFn);
  }

  function closeGap(gap: TimelineGap, seekFn?: (time: number) => void) {
    const gapDuration = gap.end - gap.start;
    if (gapDuration <= 0.02) return;

    recordSnapshot();

    clips.value.forEach((c) => {
      if (c.start >= gap.end - 0.05) {
        c.start = Math.max(0, c.start - gapDuration);
        c.end = Math.max(c.start + 0.05, c.end - gapDuration);
      }
    });

    renumberClips();
    syncSplitPointsFromClips();
    if (selectedClip.value) {
      seekFn?.(selectedClip.value.start);
    }
  }

  function closeGapBeforeClip(clip: ClipItem, seekFn?: (time: number) => void) {
    if (!clip) return;
    const sorted = [...clips.value].sort((a, b) => a.start - b.start);
    const curIdx = sorted.findIndex((c) => c.id === clip.id);
    const prevEnd = curIdx > 0 ? sorted[curIdx - 1].end : 0;
    const gapSize = clip.start - prevEnd;
    if (gapSize <= 0.02) return;

    recordSnapshot();

    for (let i = curIdx; i < sorted.length; i++) {
      sorted[i].start = Math.max(0, sorted[i].start - gapSize);
      sorted[i].end = Math.max(sorted[i].start + 0.05, sorted[i].end - gapSize);
    }
    renumberClips();
    syncSplitPointsFromClips();
    seekFn?.(clip.start);
  }

  function closeAllGaps(seekFn?: (time: number) => void) {
    const sorted = [...clips.value].sort((a, b) => a.start - b.start);
    let cursor = 0;
    recordSnapshot();

    for (let i = 0; i < sorted.length; i++) {
      const dur = sorted[i].end - sorted[i].start;
      sorted[i].start = cursor;
      sorted[i].end = cursor + dur;
      cursor += dur;
    }
    renumberClips();
    syncSplitPointsFromClips();
    if (selectedClip.value) {
      seekFn?.(selectedClip.value.start);
    }
  }

  function deleteSelectedClip() {
    if (!selectedClipId.value || clips.value.length === 0) return;
    const idx = clips.value.findIndex((c) => c.id === selectedClipId.value);
    if (idx === -1) return;

    recordSnapshot();

    clips.value.splice(idx, 1);
    renumberClips();
    syncSplitPointsFromClips();

    if (clips.value.length > 0) {
      const nextIdx = Math.min(idx, clips.value.length - 1);
      selectClip(clips.value[nextIdx].id);
    } else {
      selectedClipId.value = null;
    }
  }

  function deleteClipById(id: string) {
    selectClip(id);
    deleteSelectedClip();
  }

  function keepOnlySelectedClip() {
    if (!selectedClipId.value || clips.value.length === 0) return;
    const clip = clips.value.find((c) => c.id === selectedClipId.value);
    if (!clip) return;
    recordSnapshot();
    clips.value = [clip];
    renumberClips();
    syncSplitPointsFromClips();
    selectClip(clip.id);
  }

  function markIn(currentTime: number, totalDuration: number) {
    inPoint.value = currentTime;
    if (outPoint.value <= inPoint.value) {
      outPoint.value = totalDuration;
    }
  }

  function markOut(currentTime: number) {
    outPoint.value = currentTime;
    if (outPoint.value <= inPoint.value) {
      inPoint.value = 0;
    }
  }

  // VF-13: Normaliza, valida y restaura completamente el estado al abrir un proyecto
  function loadProjectState(
    loadedClips: any[],
    mediaDuration: number,
    preferredSelectedClipId?: string | null,
    savedInPoint?: number,
    savedOutPoint?: number
  ) {
    if (!Array.isArray(loadedClips) || loadedClips.length === 0 || mediaDuration <= 0) {
      if (mediaDuration > 0) initClips(mediaDuration);
      return;
    }

    const validClips: ClipItem[] = [];

    for (let i = 0; i < loadedClips.length; i++) {
      const raw = loadedClips[i];
      if (typeof raw !== 'object' || raw === null) continue;

      let start = typeof raw.start === 'number' && !isNaN(raw.start) ? raw.start : 0;
      let end = typeof raw.end === 'number' && !isNaN(raw.end) ? raw.end : mediaDuration;

      // Clamping al timeline
      start = Math.max(0, Math.min(start, mediaDuration - 0.05));
      end = Math.max(start + 0.05, Math.min(end, mediaDuration));

      let sourceStart = typeof raw.sourceStart === 'number' && !isNaN(raw.sourceStart) ? raw.sourceStart : start;
      let sourceEnd = typeof raw.sourceEnd === 'number' && !isNaN(raw.sourceEnd) ? raw.sourceEnd : end;

      // Clamping a la duración del archivo de medio detectado
      sourceStart = Math.max(0, Math.min(sourceStart, mediaDuration));
      sourceEnd = Math.max(sourceStart + 0.05, Math.min(sourceEnd, mediaDuration));

      const id = typeof raw.id === 'string' && raw.id ? raw.id : `clip-${i + 1}`;
      const name = typeof raw.name === 'string' && raw.name ? raw.name : `Clip ${i + 1}`;
      const color = typeof raw.color === 'string' && raw.color ? raw.color : CLIP_COLORS[i % CLIP_COLORS.length];

      validClips.push({
        id,
        name,
        start,
        end,
        sourceStart,
        sourceEnd,
        color,
      });
    }

    if (validClips.length === 0) {
      initClips(mediaDuration);
      return;
    }

    // Ordenar clips cronológicamente por inicio en timeline
    validClips.sort((a, b) => a.start - b.start);
    clips.value = validClips;

    // VF-13: Restaurar o seleccionar un clip existente válido
    if (preferredSelectedClipId && validClips.some((c) => c.id === preferredSelectedClipId)) {
      selectedClipId.value = preferredSelectedClipId;
    } else {
      selectedClipId.value = validClips[0].id;
    }

    // Sincronizar splitPoints ordenados
    syncSplitPointsFromClips();

    // Validar inPoint y outPoint contra la duración real del archivo
    const inVal = typeof savedInPoint === 'number' && !isNaN(savedInPoint) ? savedInPoint : 0;
    const outVal = typeof savedOutPoint === 'number' && !isNaN(savedOutPoint) ? savedOutPoint : mediaDuration;
    inPoint.value = Math.max(0, Math.min(inVal, mediaDuration));
    outPoint.value = Math.max(inPoint.value, Math.min(outVal, mediaDuration));

    undoStack.value = [];
    redoStack.value = [];
  }

  function setTool(tool: ActiveTool) {
    activeTool.value = tool;
  }

  return {
    clips,
    selectedClipId,
    selectedClip,
    sortedClips,
    splitPoints,
    inPoint,
    outPoint,
    timelineGaps,
    activeTool,
    isSidebarCollapsed,
    canUndo,
    canRedo,
    recordSnapshot,
    undo,
    redo,
    initClips,
    selectClip,
    splitAtTime,
    splitAtCurrentPlayhead,
    splitByInOut,
    closeGap,
    closeGapBeforeClip,
    closeAllGaps,
    deleteClipById,
    deleteSelectedClip,
    keepOnlySelectedClip,
    markIn,
    markOut,
    setTool,
    renumberClips,
    syncSplitPointsFromClips,
    loadProjectState,
  };
}
