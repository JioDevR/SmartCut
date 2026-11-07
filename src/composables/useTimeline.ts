import { ref, computed, nextTick, type Ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { ClipItem, TimelineTile, RulerMark } from '../types';
import { formatRulerTime } from '../utils/formatters';

export const TILE_WIDTH = 96;

export function useTimeline() {
  const timelineTrack = ref<HTMLElement | null>(null);
  const timelineScrollContainer = ref<HTMLElement | null>(null);

  const pixelsPerSecond = ref(30);
  const minPixelsPerSecond = 1;
  const maxPixelsPerSecond = 1500;

  const scrollLeft = ref(0);
  const viewportWidth = ref(1200);

  const thumbnailCache = ref<Record<number, string>>({});
  const sortedThumbnailKeys = computed(() => {
    return Object.keys(thumbnailCache.value).map(Number).sort((a, b) => a - b);
  });
  const audioWaveformUrl = ref('');
  const audioPeaks = ref<Uint8Array | null>(null);

  const isDraggingPlayhead = ref(false);
  const hoverTime = ref<number | null>(null);
  const hoverPixel = ref(0);
  const isHoveringTimeline = ref(false);

  let fetchThumbnailsTimer: ReturnType<typeof setTimeout> | null = null;
  let isFetchingRange = false;

  const zoomLevel = computed(() => {
    if (pixelsPerSecond.value < 15) return 1;
    if (pixelsPerSecond.value < 60) return 2;
    if (pixelsPerSecond.value < 250) return 3;
    return 4;
  });

  function getTimelineTrackWidth(duration: number) {
    const dur = duration > 0 ? duration : 10;
    return Math.max(800, dur * pixelsPerSecond.value);
  }

  function getThumbnailForTime(time: number): string | null {
    const cache = thumbnailCache.value;
    const exactKey = Math.round(time * 10) / 10;
    if (cache[exactKey]) return cache[exactKey];
    if (cache[Math.round(time)]) return cache[Math.round(time)];

    const cacheKeys = sortedThumbnailKeys.value;
    const keysLen = cacheKeys.length;
    if (keysLen === 0) return null;

    let low = 0, high = keysLen - 1;
    let closestK = cacheKeys[0];
    let minDiff = Math.abs(closestK - time);
    while (low <= high) {
      const mid = (low + high) >> 1;
      const diff = Math.abs(cacheKeys[mid] - time);
      if (diff < minDiff) {
        minDiff = diff;
        closestK = cacheKeys[mid];
      }
      if (cacheKeys[mid] < time) low = mid + 1;
      else high = mid - 1;
    }
    // Proyección visual continua (Nearest-Neighbor): la pista nunca queda vacía
    return cache[closestK] || null;
  }

  const tilesCacheMap = new Map<string, TimelineTile[]>();
  let lastThumbCacheSnapshot = '';

  function getTilesForClip(clip: ClipItem, duration: number): TimelineTile[] {
    if (!clip || duration <= 0) return [];

    const thumbKeysCount = Object.keys(thumbnailCache.value).length;
    const thumbSnap = `${thumbKeysCount}`;
    if (thumbSnap !== lastThumbCacheSnapshot) {
      tilesCacheMap.clear();
      lastThumbCacheSnapshot = thumbSnap;
    }

    const pps = pixelsPerSecond.value;
    const clipDur = Math.max(0, clip.end - clip.start);
    if (clipDur <= 0) return [];

    const srcStart = clip.sourceStart !== undefined ? clip.sourceStart : clip.start;
    const srcEnd = clip.sourceEnd !== undefined ? clip.sourceEnd : clip.end;
    const sBucket = Math.floor(scrollLeft.value / 60);
    const vWidth = Math.floor(viewportWidth.value / 100);

    const cacheKey = `${clip.id}_${clip.start.toFixed(3)}_${clip.end.toFixed(3)}_${srcStart.toFixed(3)}_${srcEnd.toFixed(3)}_${pps}_${sBucket}_${vWidth}`;
    const cached = tilesCacheMap.get(cacheKey);
    if (cached) {
      return cached;
    }

    const clipWidthPx = clipDur * pps;
    const totalClipTiles = Math.max(1, Math.ceil(clipWidthPx / TILE_WIDTH));
    const srcDur = Math.max(0.01, srcEnd - srcStart);

    const clipTimelineLeft = clip.start * pps;
    const visibleLeftInClip = Math.max(0, (scrollLeft.value - 200) - clipTimelineLeft);
    const visibleRightInClip = Math.min(clipWidthPx, (scrollLeft.value + viewportWidth.value + 200) - clipTimelineLeft);

    if (visibleRightInClip <= visibleLeftInClip && clipWidthPx > 0) {
      tilesCacheMap.set(cacheKey, []);
      return [];
    }

    const startIdx = Math.max(0, Math.floor(visibleLeftInClip / TILE_WIDTH));
    const endIdx = Math.min(totalClipTiles, Math.ceil(visibleRightInClip / TILE_WIDTH));

    const tiles: TimelineTile[] = [];
    for (let i = startIdx; i < endIdx; i++) {
      const pixelLeft = i * TILE_WIDTH;
      const tileWidthActual = Math.min(TILE_WIDTH, Math.max(1, clipWidthPx - pixelLeft));
      const tileCenterPx = pixelLeft + tileWidthActual / 2;
      const progress = clipWidthPx > 0 ? Math.min(1.0, Math.max(0.0, tileCenterPx / clipWidthPx)) : 0;
      const tileTime = Math.min(srcEnd, srcStart + progress * srcDur);
      tiles.push({
        index: i,
        pixelLeft,
        width: TILE_WIDTH,
        time: tileTime,
        image: getThumbnailForTime(tileTime),
      });
    }

    if (tilesCacheMap.size > 250) {
      tilesCacheMap.clear();
    }
    tilesCacheMap.set(cacheKey, tiles);
    return tiles;
  }

  function computeRulerMarks(duration: number, fps: number = 30): RulerMark[] {
    const dur = duration > 0 ? duration : 10;
    const pps = pixelsPerSecond.value;

    let majorInterval = 10;
    let minorInterval = 2;

    if (pps >= 400) {
      majorInterval = 1.0;
      minorInterval = 1.0 / fps;
    } else if (pps >= 150) {
      majorInterval = 1.0;
      minorInterval = 5.0 / fps;
    } else if (pps >= 60) {
      majorInterval = 2.0;
      minorInterval = 0.5;
    } else if (pps >= 25) {
      majorInterval = 5.0;
      minorInterval = 1.0;
    } else if (pps >= 10) {
      majorInterval = 15.0;
      minorInterval = 3.0;
    } else if (pps >= 3) {
      majorInterval = 60.0;
      minorInterval = 15.0;
    } else {
      majorInterval = 300.0;
      minorInterval = 60.0;
    }

    const sLeft = scrollLeft.value;
    const vWidth = viewportWidth.value;
    const startSec = Math.max(0, Math.floor((sLeft - 300) / pps / minorInterval) * minorInterval);
    const endSec = Math.min(dur, Math.ceil((sLeft + vWidth + 300) / pps / minorInterval) * minorInterval);

    const marks: RulerMark[] = [];
    const step = Math.max(0.001, minorInterval);

    for (let t = startSec; t <= endSec; t += step) {
      const isMajor = Math.abs(t % majorInterval) < 0.005 || Math.abs((t % majorInterval) - majorInterval) < 0.005;
      const px = t * pps;
      marks.push({
        time: Math.round(t * 1000) / 1000,
        pixel: px,
        isMajor,
        label: isMajor ? formatRulerTime(t, zoomLevel.value) : null,
      });
    }

    return marks;
  }

  function setZoom(newPps: number, anchorTime: number | null = null, requestThumbsFn?: () => void) {
    const clamped = Math.max(minPixelsPerSecond, Math.min(maxPixelsPerSecond, newPps));
    if (clamped === pixelsPerSecond.value) return;

    const container = timelineScrollContainer.value;
    let viewportOffset = 0;
    if (container && anchorTime !== null) {
      const oldAnchorPx = anchorTime * pixelsPerSecond.value;
      viewportOffset = oldAnchorPx - container.scrollLeft;
    }

    pixelsPerSecond.value = clamped;

    nextTick(() => {
      if (container && anchorTime !== null) {
        const newAnchorPx = anchorTime * pixelsPerSecond.value;
        container.scrollLeft = newAnchorPx - viewportOffset;
      }
      requestThumbsFn?.();
    });
  }

  function zoomIn(currentTime: number, requestThumbsFn?: () => void) {
    setZoom(pixelsPerSecond.value * 1.4, currentTime, requestThumbsFn);
  }

  function zoomOut(currentTime: number, requestThumbsFn?: () => void) {
    setZoom(pixelsPerSecond.value / 1.4, currentTime, requestThumbsFn);
  }

  function fitToScreen(duration: number, requestThumbsFn?: () => void) {
    const container = timelineScrollContainer.value;
    if (!container || duration <= 0) return;
    const availableWidth = container.clientWidth - 48;
    const pps = Math.max(minPixelsPerSecond, availableWidth / duration);
    pixelsPerSecond.value = pps;
    container.scrollLeft = 0;
    nextTick(() => requestThumbsFn?.());
  }

  function onTimelineScroll(e: Event, requestThumbsFn?: () => void) {
    const target = e.target as HTMLElement;
    if (target) {
      scrollLeft.value = target.scrollLeft;
      viewportWidth.value = target.clientWidth || 1200;
      requestThumbsFn?.();
    }
  }

  function onTimelineWheel(e: WheelEvent, currentTime: number, requestThumbsFn?: () => void) {
    if (e.ctrlKey || e.altKey || e.metaKey) {
      e.preventDefault();
      const container = timelineScrollContainer.value;
      let anchorTime = currentTime;

      if (container && timelineTrack.value) {
        const rect = timelineTrack.value.getBoundingClientRect();
        const mouseX = e.clientX - rect.left;
        anchorTime = mouseX / pixelsPerSecond.value;
      }

      const zoomFactor = e.deltaY < 0 ? 1.2 : 0.83;
      setZoom(pixelsPerSecond.value * zoomFactor, anchorTime, requestThumbsFn);
    }
  }

  function requestVisibleThumbnails(
    filePath: string,
    duration: number,
    isPlaying?: boolean | (() => boolean) | Ref<boolean>
  ) {
    const checkIsBusy = () => {
      if (isDraggingPlayhead.value) return true;
      if (typeof isPlaying === 'function') return isPlaying();
      if (isPlaying && typeof isPlaying === 'object' && 'value' in isPlaying) {
        return (isPlaying as Ref<boolean>).value;
      }
      return !!isPlaying;
    };

    if (!filePath || duration <= 0 || checkIsBusy()) return;
    if (fetchThumbnailsTimer) clearTimeout(fetchThumbnailsTimer);

    fetchThumbnailsTimer = setTimeout(async () => {
      if (isFetchingRange || checkIsBusy()) return;
      const pps = pixelsPerSecond.value;
      const container = timelineScrollContainer.value;
      if (container) {
        scrollLeft.value = container.scrollLeft;
        viewportWidth.value = container.clientWidth || 1200;
      }

      const startIdx = Math.max(0, Math.floor((scrollLeft.value - 200) / TILE_WIDTH));
      const totalTiles = Math.max(1, Math.ceil((duration * pps) / TILE_WIDTH));
      const endIdx = Math.min(totalTiles, Math.ceil((scrollLeft.value + viewportWidth.value + 200) / TILE_WIDTH));

      if (endIdx <= startIdx) return;

      const startTime = Math.max(0, (startIdx * TILE_WIDTH) / pps);
      const endTime = Math.min(duration, (endIdx * TILE_WIDTH) / pps);
      if (endTime <= startTime) return;

      const visibleTiles = endIdx - startIdx;
      const count = Math.min(20, Math.max(4, visibleTiles));

      isFetchingRange = true;
      try {
        const newThumbs = await invoke<Array<{ time: number; data_url: string }>>('get_thumbnails_for_range', {
          path: filePath,
          startTime,
          endTime,
          count,
        });

        if (Array.isArray(newThumbs) && newThumbs.length > 0) {
          const updated = { ...thumbnailCache.value };
          for (const item of newThumbs) {
            const k = Math.round(item.time * 10) / 10;
            updated[k] = item.data_url;
          }
          thumbnailCache.value = updated;
        }
      } catch (err) {
        console.warn('Aviso miniaturas dinámicas:', err);
      } finally {
        isFetchingRange = false;
      }
    }, 250);
  }

  async function loadVisualizations(filePath: string, duration: number) {
    if (!filePath || duration <= 0) return;
    thumbnailCache.value = {};
    audioWaveformUrl.value = '';
    audioPeaks.value = null;

    // 1. Extraer 24 miniaturas base distribuidas que cubren todo el video en paralelo
    invoke<Array<{ time: number; data_url: string }>>('get_timeline_thumbnails', {
      path: filePath,
      duration,
      count: 24,
    })
      .then((thumbs) => {
        if (Array.isArray(thumbs) && thumbs.length > 0) {
          const initial: Record<number, string> = {};
          for (const t of thumbs) {
            const k = Math.round(t.time * 10) / 10;
            initial[k] = t.data_url;
          }
          thumbnailCache.value = initial;
          // Refinar de inmediato el área visible actual
          requestVisibleThumbnails(filePath, duration);
        }
      })
      .catch((e) => console.warn('Aviso miniaturas:', e));

    // 2. Extraer picos de audio de alta velocidad (Peak Data 100Hz) en segundo plano
    invoke<number[]>('get_audio_peaks', { path: filePath })
      .then((data) => {
        if (data && data.length > 0) {
          audioPeaks.value = new Uint8Array(data);
        }
      })
      .catch((e) => console.warn('Aviso picos de audio:', e));

    // 3. Fallback de visualización
    invoke<string>('get_audio_visualization', { path: filePath, mode: 'waves' })
      .then((url) => {
        if (url) {
          audioWaveformUrl.value = url;
        }
      })
      .catch((e) => console.warn('Aviso audio:', e));
  }

  return {
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
    zoomLevel,
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
  };
}
