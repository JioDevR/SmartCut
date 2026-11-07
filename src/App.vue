<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, provide } from 'vue';
import type { ClipItem, TimelineGap, ContextMenuState } from './types';
import { useVideoPlayer } from './composables/useVideoPlayer';
import { useClipsManager } from './composables/useClipsManager';
import { useTimeline } from './composables/useTimeline';
import { useExport } from './composables/useExport';
import { useClipDrag } from './composables/useClipDrag';
import { useKeyboardShortcuts } from './composables/useKeyboardShortcuts';
import { useNativeDrop } from './composables/useNativeDrop';

// Importación de Componentes Modulares
import HeaderBar from './components/header/HeaderBar.vue';
import ClipsSidebar from './components/sidebar/ClipsSidebar.vue';
import CinemaPlayer from './components/player/CinemaPlayer.vue';
import TimelineDock from './components/timeline/TimelineDock.vue';
import ExportModal from './components/export/ExportModal.vue';
import AboutModal from './components/common/AboutModal.vue';
import ClipContextMenu from './components/common/ClipContextMenu.vue';

// 1. Inicialización de Composables Core
const {
  videoPlayer,
  currentFilePath,
  videoSrc,
  isPlaying,
  isBuffering,
  currentTime,
  duration,
  volume,
  isMuted,
  playbackRate,
  metadata,
  isOpeningFile,
  errorMessage,
  isCodecIncompatible,
  isGeneratingPreview,
  audioLevelL,
  audioLevelR,
  audioDbText,
  loadVideoFile,
  openNativeFileDialog,
  createCompatiblePreview,
  onVideoError,
  onLoadedData,
  onLoadedMetadata,
  seekTo,
  fastSeekTo,
  onSeeked,
  stepFrame,
  stepSeconds,
  togglePlay,
  onPlayStarted,
  onPlayPaused,
  toggleMute,
  setVolume,
  onTimeUpdate,
  setAudioPeaks,
} = useVideoPlayer();

const {
  clips,
  selectedClipId,
  selectedClip,
  sortedClips,
  splitPoints,
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
  closeGap,
  closeGapBeforeClip,
  closeAllGaps,
  deleteClipById,
  deleteSelectedClip,
  keepOnlySelectedClip,
  setTool,
  renumberClips,
  syncSplitPointsFromClips,
} = useClipsManager();

const {
  timelineTrack,
  timelineScrollContainer,
  pixelsPerSecond,
  minPixelsPerSecond,
  maxPixelsPerSecond,
  scrollLeft,
  viewportWidth,
  audioWaveformUrl,
  audioPeaks,
  isDraggingPlayhead,
  hoverTime,
  hoverPixel,
  isHoveringTimeline,
  getTimelineTrackWidth,
  getTilesForClip,
  computeRulerMarks,
  setZoom,
  zoomIn,
  zoomOut,
  fitToScreen,
  onTimelineScroll,
  onTimelineWheel,
  requestVisibleThumbnails,
  loadVisualizations,
} = useTimeline();

watch(audioPeaks, (peaks) => {
  setAudioPeaks(peaks);
}, { immediate: true });

const {
  isExportModalOpen,
  exportOutputPath,
  exportMode,
  exportScope,
  setExportScope,
  availableEncoders,
  selectedEncoder,
  isExporting,
  exportProgressPercent,
  exportStageText,
  exportFps,
  exportSpeed,
  exportFinishedSuccess,
  exportErrorMessage,
  containerFormat,
  crf,
  preset,
  audioOnly,
  audioFormat,
  planSummary,
  updateOutputExtension,
  openExportModal,
  browseExportSavePath,
  startExport,
  cancelExport,
  openExportedFolder,
  openExportedFile,
  cleanupExportListener,
} = useExport();

// 2. Referencias de Componentes y Estado Local
const cinemaPlayerRef = ref<InstanceType<typeof CinemaPlayer> | null>(null);
const timelineDockRef = ref<InstanceType<typeof TimelineDock> | null>(null);
const isAboutModalOpen = ref(false);

const contextMenu = ref<ContextMenuState>({
  visible: false,
  x: 0,
  y: 0,
  clip: null,
});

// Sincronizar referencia del elemento de video nativo
watch(
  () => cinemaPlayerRef.value?.videoElement,
  (el) => {
    if (el) videoPlayer.value = el;
  },
  { immediate: true }
);

// Sincronizar referencias del timeline
watch(
  () => timelineDockRef.value?.timelineTrack,
  (track) => {
    if (track) timelineTrack.value = track;
  },
  { immediate: true }
);
watch(
  () => timelineDockRef.value?.timelineScrollContainer,
  (sc) => {
    if (sc) timelineScrollContainer.value = sc;
  },
  { immediate: true }
);

// 3. Cálculos reactivos
const isPlayheadInGap = computed(() => {
  if (!currentFilePath.value || duration.value <= 0 || clips.value.length === 0) return false;
  if (isDraggingPlayhead.value) return false;
  const t = currentTime.value;
  const inClip = clips.value.some((c) => t >= c.start - 0.005 && t <= c.end + 0.005);
  return !inClip;
});

const rulerMarks = computed(() => {
  return computeRulerMarks(duration.value, metadata.value.fps);
});

const timelineTrackWidth = computed(() => {
  return getTimelineTrackWidth(duration.value);
});

const playheadPixelPosition = computed(() => {
  return currentTime.value * pixelsPerSecond.value;
});

// Provide de estado de cabezal y hover para TimelinePlayhead y TimelineToolbar
provide('timelinePlayheadState', {
  currentTime,
  playheadPixelPosition,
  hoverTime,
  hoverPixel,
  isHoveringTimeline,
});

function handleGetTilesForClip(clip: ClipItem) {
  return getTilesForClip(clip, duration.value);
}

// 4. Adaptadores de carga y eventos
function handleLoadVideo(filePath: string) {
  loadVideoFile(filePath, {
    onInitClips: (dur) => initClips(dur),
    onLoadVisualizations: (path, dur) => loadVisualizations(path, dur),
    onFitToScreen: () => fitToScreen(duration.value, () => requestVisibleThumbnails(currentFilePath.value, duration.value, () => isPlaying.value)),
  });
}

function handleOpenFile() {
  openNativeFileDialog({
    onInitClips: (dur) => initClips(dur),
    onLoadVisualizations: (path, dur) => loadVisualizations(path, dur),
    onFitToScreen: () => fitToScreen(duration.value, () => requestVisibleThumbnails(currentFilePath.value, duration.value, () => isPlaying.value)),
  });
}

function handleTogglePlay() {
  togglePlay(() => sortedClips.value);
}

function handleSeek(targetTime: number, preferredClipId?: string | null) {
  seekTo(targetTime, () => sortedClips.value, preferredClipId);
}

function handleStepFrame(delta: number) {
  stepFrame(delta, () => sortedClips.value);
}

function handleStepSeconds(delta: number) {
  stepSeconds(delta, () => sortedClips.value);
}

function handleCutAtTime(t: number) {
  splitAtTime(t, duration.value, (newT) => handleSeek(newT));
}

function handleSplitAtPlayhead() {
  splitAtCurrentPlayhead(currentTime.value, duration.value, (newT) => handleSeek(newT));
}

function handleCloseGap(gap: TimelineGap) {
  closeGap(gap, (newT) => handleSeek(newT));
}

function handleSelectClip(clip: ClipItem) {
  selectClip(clip.id);
  handleSeek(clip.start, clip.id);
}

function handlePlayClip(clip: ClipItem) {
  selectClip(clip.id);
  handleSeek(clip.start, clip.id);
  if (videoPlayer.value) {
    videoPlayer.value.play().catch(() => {});
  }
}

// 5. Interacción del puntero en el timeline
let trackRect: DOMRect | null = null;
let pendingMoveRaf: number | null = null;
let lastMoveEvent: PointerEvent | null = null;

const {
  isDraggingClip,
  draggingClipId,
  onClipPointerDown,
  onClipPointerMove,
  onClipPointerUp,
  onTrimHandleDown,
} = useClipDrag({
  clips,
  duration,
  pixelsPerSecond,
  activeTool,
  timelineTrack,
  selectClip,
  handleCutAtTime,
  handleSeek,
  fastSeekTo: (t) => fastSeekTo(t, () => sortedClips.value),
  recordSnapshot,
  renumberClips,
  syncSplitPointsFromClips,
});

function onTrackPointerDown(e: PointerEvent) {
  if (!timelineTrack.value || e.button !== 0) return;
  trackRect = timelineTrack.value.getBoundingClientRect();
  const clickX = Math.max(0, Math.min(e.clientX - trackRect.left, trackRect.width));
  const targetTime = clickX / pixelsPerSecond.value;

  if (activeTool.value === 'cut') {
    e.preventDefault();
    handleCutAtTime(targetTime);
    return;
  }

  const target = e.target as HTMLElement;
  if (target.closest('.clip-timeline-item') || target.closest('.handle')) {
    return;
  }

  e.preventDefault();
  timelineTrack.value.setPointerCapture(e.pointerId);
  isDraggingPlayhead.value = true;
  currentTime.value = Math.max(0, Math.min(targetTime, duration.value));
  handleSeek(targetTime);
}

function onTrackPointerMove(e: PointerEvent) {
  if (!timelineTrack.value) return;
  lastMoveEvent = e;
  if (pendingMoveRaf) return;

  pendingMoveRaf = requestAnimationFrame(() => {
    pendingMoveRaf = null;
    if (!lastMoveEvent || !timelineTrack.value) return;

    const rect = trackRect || timelineTrack.value.getBoundingClientRect();
    const mouseX = Math.max(0, Math.min(lastMoveEvent.clientX - rect.left, rect.width));

    hoverPixel.value = mouseX;
    hoverTime.value = mouseX / pixelsPerSecond.value;
    isHoveringTimeline.value = true;

    if (isDraggingPlayhead.value) {
      const targetTime = Math.max(0, Math.min(mouseX / pixelsPerSecond.value, duration.value));
      currentTime.value = targetTime;
      fastSeekTo(targetTime, () => sortedClips.value);
    }
  });
}

function onTrackPointerUp(e: PointerEvent) {
  if (pendingMoveRaf) {
    cancelAnimationFrame(pendingMoveRaf);
    pendingMoveRaf = null;
  }
  trackRect = null;
  if (isDraggingPlayhead.value && timelineTrack.value) {
    try {
      timelineTrack.value.releasePointerCapture(e.pointerId);
    } catch (_) {}
    isDraggingPlayhead.value = false;
    handleSeek(currentTime.value);
  }
}

function onTrackPointerLeave() {
  if (!isDraggingPlayhead.value) {
    if (pendingMoveRaf) {
      cancelAnimationFrame(pendingMoveRaf);
      pendingMoveRaf = null;
    }
    isHoveringTimeline.value = false;
    trackRect = null;
  }
}

function handleZoomIn() {
  zoomIn(currentTime.value, () => requestVisibleThumbnails(currentFilePath.value, duration.value, () => isPlaying.value));
}

function handleZoomOut() {
  zoomOut(currentTime.value, () => requestVisibleThumbnails(currentFilePath.value, duration.value, () => isPlaying.value));
}

function handleSetZoom(pps: number) {
  setZoom(pps, currentTime.value, () => requestVisibleThumbnails(currentFilePath.value, duration.value, () => isPlaying.value));
}

function handleTimelineWheel(event: WheelEvent) {
  onTimelineWheel(event, currentTime.value, () => requestVisibleThumbnails(currentFilePath.value, duration.value, () => isPlaying.value));
}

function handleTimelineScroll(event: Event) {
  onTimelineScroll(event, () => requestVisibleThumbnails(currentFilePath.value, duration.value, () => isPlaying.value));
}

// 6. Drag & Drop y Menú Contextual
const { isDraggingFile, onHtmlDragOver, onHtmlDragLeave, onHtmlDrop } = useNativeDrop({
  onFileDrop: (path) => handleLoadVideo(path),
});

function openContextMenu(e: MouseEvent, clip: ClipItem) {
  selectClip(clip.id);
  const x = Math.min(e.clientX, window.innerWidth - 240);
  const y = Math.min(e.clientY, window.innerHeight - 240);
  contextMenu.value = {
    visible: true,
    x: Math.max(10, x),
    y: Math.max(10, y),
    clip,
  };
}

function closeContextMenu() {
  contextMenu.value.visible = false;
}

function onGlobalClick() {
  if (contextMenu.value.visible) {
    closeContextMenu();
  }
}

// 7. Atajos de Teclado Centralizados
useKeyboardShortcuts({
  onUndo: () => undo((t) => handleSeek(t)),
  onRedo: () => redo((t) => handleSeek(t)),
  onOpenFile: handleOpenFile,
  onOpenExport: () => {
    if (currentFilePath.value) openExportModal(currentFilePath.value, metadata.value, sortedClips.value, selectedClip.value);
  },
  onZoomIn: () => zoomIn(currentTime.value, () => requestVisibleThumbnails(currentFilePath.value, duration.value, () => isPlaying.value)),
  onZoomOut: () => zoomOut(currentTime.value, () => requestVisibleThumbnails(currentFilePath.value, duration.value, () => isPlaying.value)),
  onStepFrame: handleStepFrame,
  onStepSeconds: handleStepSeconds,
  onTogglePlay: handleTogglePlay,
  onSetTool: (t) => setTool(t),
  onDeleteSelectedClip: deleteSelectedClip,
  onSplitAtPlayhead: handleSplitAtPlayhead,
  onCloseContextMenu: closeContextMenu,
  isContextMenuVisible: () => contextMenu.value.visible,
  hasSelectedClip: () => !!selectedClipId.value,
});

onMounted(() => {
  window.addEventListener('click', onGlobalClick);
});

onUnmounted(() => {
  window.removeEventListener('click', onGlobalClick);
  cleanupExportListener();
});
</script>

<template>
  <div
    class="workspace"
    @dragover="onHtmlDragOver"
    @dragleave="onHtmlDragLeave"
    @drop="onHtmlDrop"
  >
    <!-- OVERLAY DRAG & DROP -->
    <div v-if="isDraggingFile" class="drag-drop-overlay">
      <div class="drag-drop-box">
        <span class="material-icons-round drop-icon">cloud_upload</span>
        <h3>Suelta el archivo de video aquí</h3>
        <p>SmartCut cargará el video instantáneamente para edición</p>
      </div>
    </div>

    <!-- 1. BARRA SUPERIOR MINIMALISTA -->
    <HeaderBar
      :current-file-path="currentFilePath"
      :metadata="metadata"
      :is-opening-file="isOpeningFile"
      :error-message="errorMessage"
      :is-codec-incompatible="isCodecIncompatible"
      :is-generating-preview="isGeneratingPreview"
      :can-export="!!currentFilePath && sortedClips.length > 0"
      @open-file="handleOpenFile"
      @export="openExportModal(currentFilePath, metadata, sortedClips, selectedClip)"
      @open-about="isAboutModalOpen = true"
      @generate-preview="createCompatiblePreview"
      @dismiss-error="errorMessage = ''"
    />

    <!-- 2. ETAPA CENTRAL: SIDEBAR Y CINEMA PLAYER -->
    <div class="stage-container">
      <ClipsSidebar
        v-if="videoSrc"
        :clips="clips"
        :selected-clip-id="selectedClipId"
        :metadata="metadata"
        :is-collapsed="isSidebarCollapsed"
        :is-playing="isPlaying"
        :audio-level-l="audioLevelL"
        :audio-level-r="audioLevelR"
        :audio-db-text="audioDbText"
        :volume="volume"
        :is-muted="isMuted"
        @toggle-collapse="isSidebarCollapsed = !isSidebarCollapsed"
        @select-clip="handleSelectClip"
        @play-clip="handlePlayClip"
        @delete-clip="deleteClipById"
        @context-menu="openContextMenu"
        @toggle-mute="toggleMute"
        @set-volume="setVolume"
      />

      <CinemaPlayer
        ref="cinemaPlayerRef"
        :video-src="videoSrc"
        :is-playing="isPlaying"
        :is-playhead-in-gap="isPlayheadInGap"
        @open-file="handleOpenFile"
        @toggle-play="handleTogglePlay"
        @time-update="onTimeUpdate(() => sortedClips, isDraggingPlayhead)"
        @loaded-metadata="onLoadedMetadata((dur) => initClips(dur))"
        @loaded-data="onLoadedData"
        @seeked="onSeeked"
        @error="onVideoError"
        @play-started="onPlayStarted(() => sortedClips)"
        @play-paused="onPlayPaused(() => sortedClips)"
      />
    </div>

    <!-- 3. DOCK DE LÍNEA DE TIEMPO INFERIOR -->
    <TimelineDock
      ref="timelineDockRef"
      :duration="duration"
      :pixels-per-second="pixelsPerSecond"
      :min-pixels-per-second="minPixelsPerSecond"
      :max-pixels-per-second="maxPixelsPerSecond"
      :active-tool="activeTool"
      :clips="clips"
      :selected-clip-id="selectedClipId"
      :split-points="splitPoints"
      :timeline-gaps="timelineGaps"
      :audio-waveform-url="audioWaveformUrl"
      :audio-peaks="audioPeaks"
      :scroll-left="scrollLeft"
      :viewport-width="viewportWidth"
      :metadata="metadata"
      :ruler-marks="rulerMarks"
      :timeline-track-width="timelineTrackWidth"
      :is-playing="isPlaying"
      :can-undo="canUndo"
      :can-redo="canRedo"
      :get-tiles-for-clip="handleGetTilesForClip"
      @set-tool="setTool"
      @split-at-playhead="handleSplitAtPlayhead"
      @cut-at-time="handleCutAtTime"
      @undo="undo((t) => handleSeek(t))"
      @redo="redo((t) => handleSeek(t))"
      @toggle-play="handleTogglePlay"
      @step-frame="handleStepFrame"
      @step-seconds="handleStepSeconds"
      @trim-handle-down="onTrimHandleDown"
      @select-clip="selectClip"
      @jump-to-clip-start="handleSelectClip"
      @close-gap="handleCloseGap"
      @context-menu="openContextMenu"
      @set-zoom="handleSetZoom"
      @zoom-in="handleZoomIn"
      @zoom-out="handleZoomOut"
      @track-pointer-down="onTrackPointerDown"
      @track-pointer-move="onTrackPointerMove"
      @track-pointer-up="onTrackPointerUp"
      @track-pointer-leave="onTrackPointerLeave"
      @track-wheel="handleTimelineWheel"
      @track-scroll="handleTimelineScroll"
      @clip-pointer-down="onClipPointerDown"
      @clip-pointer-move="onClipPointerMove"
      @clip-pointer-up="onClipPointerUp"
    />

    <!-- 4. MODALES Y MENÚS FLOTANTES -->
    <ExportModal
      :is-open="isExportModalOpen"
      :is-exporting="isExporting"
      :export-finished-success="exportFinishedSuccess"
      :export-progress-percent="exportProgressPercent"
      :export-stage-text="exportStageText"
      :export-fps="exportFps"
      :export-speed="exportSpeed"
      :export-error-message="exportErrorMessage"
      :export-output-path="exportOutputPath"
      :export-mode="exportMode"
      :export-scope="exportScope"
      :selected-clip="selectedClip"
      :available-encoders="availableEncoders"
      :selected-encoder="selectedEncoder"
      :container-format="containerFormat"
      :crf="crf"
      :preset="preset"
      :audio-only="audioOnly"
      :audio-format="audioFormat"
      :plan-summary="planSummary"
      :sorted-clips="sortedClips"
      :metadata="metadata"
      @close="isExportModalOpen = false"
      @update:export-mode="exportMode = $event"
      @update:export-scope="setExportScope($event)"
      @update:selected-encoder="selectedEncoder = $event"
      @update:container-format="containerFormat = $event; updateOutputExtension()"
      @update:crf="crf = $event"
      @update:preset="preset = $event"
      @update:audio-only="audioOnly = $event; updateOutputExtension()"
      @update:audio-format="audioFormat = $event; updateOutputExtension()"
      @browse-path="browseExportSavePath"
      @start-export="startExport(currentFilePath, sortedClips, metadata.video_codec, selectedClip)"
      @cancel-export="cancelExport"
      @open-file="openExportedFile"
      @open-folder="openExportedFolder"
    />

    <AboutModal
      :is-open="isAboutModalOpen"
      @close="isAboutModalOpen = false"
    />

    <ClipContextMenu
      :visible="contextMenu.visible"
      :x="contextMenu.x"
      :y="contextMenu.y"
      :clip="contextMenu.clip"
      :fps="metadata.fps"
      @play="contextMenu.clip && handlePlayClip(contextMenu.clip); closeContextMenu();"
      @close-gap-before="contextMenu.clip && closeGapBeforeClip(contextMenu.clip, (t) => handleSeek(t)); closeContextMenu();"
      @close-all-gaps="closeAllGaps((t) => handleSeek(t)); closeContextMenu();"
      @keep-only="keepOnlySelectedClip(); closeContextMenu();"
      @delete="deleteSelectedClip(); closeContextMenu();"
      @close="closeContextMenu"
    />
  </div>
</template>

<style scoped>
.workspace {
  width: 100vw;
  height: 100vh;
  display: flex;
  flex-direction: column;
  background-color: #07090e;
  overflow: hidden;
  position: relative;
  font-family: var(--font-ui);
  color: var(--text-primary);
}

.stage-container {
  flex: 1;
  display: flex;
  overflow: hidden;
  position: relative;
  min-height: 0;
}

/* Overlay drag and drop nativo */
.drag-drop-overlay {
  position: absolute;
  inset: 0;
  background-color: rgba(16, 185, 129, 0.08);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
}

.drag-drop-box {
  background-color: rgba(14, 18, 24, 0.96);
  border: 2px dashed #10b981;
  border-radius: var(--radius-xl);
  padding: 40px 60px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  box-shadow: 0 20px 50px rgba(0, 0, 0, 0.7);
  animation: pulse-border 1.8s infinite;
}

.drop-icon {
  font-size: 48px;
  color: #34d399;
}

.drag-drop-box h3 {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-pure);
}

.drag-drop-box p {
  font-size: 13px;
  color: var(--text-secondary);
}

@keyframes pulse-border {
  0% {
    border-color: #10b981;
    box-shadow: 0 0 16px rgba(16, 185, 129, 0.2);
  }
  50% {
    border-color: #34d399;
    box-shadow: 0 0 24px rgba(52, 211, 153, 0.35);
  }
  100% {
    border-color: #10b981;
    box-shadow: 0 0 16px rgba(16, 185, 129, 0.2);
  }
}
</style>
