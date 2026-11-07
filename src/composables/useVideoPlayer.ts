import { ref, computed, nextTick } from 'vue';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import type { VideoMetadata, ClipItem } from '../types';

export function useVideoPlayer() {
  const videoPlayer = ref<HTMLVideoElement | null>(null);

  const currentFilePath = ref('');
  const videoSrc = ref('');
  const isPlaying = ref(false);
  const isSeeking = ref(false);
  const isBuffering = ref(false);
  const currentTime = ref(0);
  const duration = ref(0);
  const volume = ref(0.85);
  const isMuted = ref(false);
  const playbackRate = ref(1.0);
  const loopEnabled = ref(false);

  const metadata = ref<VideoMetadata>({
    file_name: '',
    file_path: '',
    file_size: 0,
    duration: 0,
    width: 1920,
    height: 1080,
    fps: 30,
    format: '',
    video_codec: '',
    audio_codec: '',
  });

  const isOpeningFile = ref(false);
  const errorMessage = ref('');
  const isCodecIncompatible = ref(false);
  const isGeneratingPreview = ref(false);
  const previewProxyActive = ref(false);

  // VU Meter reactivo sincronizado con picos de audio (sin secuestrar el pipeline de audio)
  const audioLevelL = ref(0);
  const audioLevelR = ref(0);
  const audioDbText = ref('-inf dBFS');

  let audioPeaksData: Uint8Array | null = null;

  let queuedSeekTime: number | null = null;
  let queuedSeekFast = false;
  let playbackRafId: number | null = null;
  // VF-12: Rastreo del clip activo durante la reproducción
  const activePlaybackClipId = ref<string | null>(null);

  async function loadVideoFile(
    filePath: string,
    callbacks?: {
      onInitClips?: (dur: number) => void;
      onLoadVisualizations?: (path: string, dur: number) => void;
      onFitToScreen?: () => void;
    }
  ) {
    if (!filePath) return;
    errorMessage.value = '';
    try {
      if (videoPlayer.value) {
        videoPlayer.value.pause();
      }
      currentFilePath.value = filePath;
      isCodecIncompatible.value = false;
      previewProxyActive.value = false;

      // Metadatos ricos mediante ffprobe
      const meta = await invoke<VideoMetadata>('get_video_metadata', { path: filePath });
      metadata.value = meta;
      if (meta.duration > 0) {
        duration.value = meta.duration;
        callbacks?.onInitClips?.(meta.duration);
      }

      // URL de streaming HTTP local con soporte nativo de rangos 206
      let streamUrl = '';
      try {
        streamUrl = await invoke<string>('get_video_stream_url', { path: filePath });
      } catch (e) {
        console.warn('Fallo get_video_stream_url, recurriendo a convertFileSrc:', e);
        streamUrl = convertFileSrc(filePath);
      }
      videoSrc.value = streamUrl;

      // Cargar miniaturas y audio en segundo plano
      if (callbacks?.onLoadVisualizations) {
        callbacks.onLoadVisualizations(filePath, meta.duration);
      }

      nextTick(() => {
        callbacks?.onFitToScreen?.();
        if (videoPlayer.value) {
          videoPlayer.value.currentTime = 0;
          currentTime.value = 0;
          videoPlayer.value.load();
        }
      });
    } catch (err) {
      errorMessage.value = `Error al abrir el video: ${err}`;
      console.error('Error cargando video:', err);
    }
  }

  async function openNativeFileDialog(
    callbacks?: {
      onInitClips?: (dur: number) => void;
      onLoadVisualizations?: (path: string, dur: number) => void;
      onFitToScreen?: () => void;
    }
  ) {
    isOpeningFile.value = true;
    try {
      const selected = await invoke<string | null>('pick_video_file');
      if (selected) {
        await loadVideoFile(selected, callbacks);
      }
    } catch (err) {
      errorMessage.value = `Error en diálogo nativo: ${err}`;
    } finally {
      isOpeningFile.value = false;
    }
  }

  async function createCompatiblePreview() {
    if (!currentFilePath.value) return;
    isGeneratingPreview.value = true;
    errorMessage.value = '';
    try {
      const previewPath = await invoke<string>('generate_compatible_preview', { path: currentFilePath.value });
      const streamUrl = await invoke<string>('get_video_stream_url', { path: previewPath });
      videoSrc.value = streamUrl;
      previewProxyActive.value = true;
      isCodecIncompatible.value = false;
      nextTick(() => {
        if (videoPlayer.value) {
          videoPlayer.value.load();
          videoPlayer.value.currentTime = 0.001;
        }
      });
    } catch (err) {
      errorMessage.value = `Error generando vista previa compatible: ${err}`;
    } finally {
      isGeneratingPreview.value = false;
    }
  }

  function onVideoError(e: Event) {
    const err = videoPlayer.value?.error;
    console.error('Error en elemento <video>:', err, e);
    if (err) {
      if (err.code === 4) {
        isCodecIncompatible.value = true;
        errorMessage.value =
          'Fallo en el reproductor: Formato o códec no soportado directamente por el navegador (MKV/HEVC o falta gst-plugins-good en Linux). Puedes generar una vista previa compatible con 1 clic.';
      } else {
        const codeMap: Record<number, string> = {
          1: 'La reproducción fue abortada por el usuario.',
          2: 'Error de red al transmitir los datos del video.',
          3: 'Error de decodificación: el reproductor no pudo procesar este cuadro.',
        };
        errorMessage.value = `Fallo en el reproductor: ${codeMap[err.code] || 'Código ' + err.code}`;
      }
    }
  }

  function onLoadedData() {
    if (videoPlayer.value && videoPlayer.value.currentTime === 0) {
      videoPlayer.value.currentTime = 0.001;
    }
  }

  function onLoadedMetadata(onInitClips?: (dur: number) => void) {
    if (!videoPlayer.value) return;
    const v = videoPlayer.value;
    if (metadata.value.duration > 0) {
      duration.value = metadata.value.duration;
    } else if (v.duration && !isNaN(v.duration) && isFinite(v.duration) && v.duration > 0) {
      duration.value = v.duration;
    }
    if (v.videoWidth) metadata.value.width = v.videoWidth;
    if (v.videoHeight) metadata.value.height = v.videoHeight;
    if (duration.value > 0) {
      onInitClips?.(duration.value);
    }
  }

  // VF-12: Resuelve el tiempo de decodificación en base al clip activo o preferido
  function resolveSourceTime(clampedTime: number, clipsList: ClipItem[], preferredClipId?: string | null): number | null {
    if (clipsList.length === 0) return clampedTime;
    let clip: ClipItem | null = null;
    if (preferredClipId) {
      clip = clipsList.find((c) => c.id === preferredClipId && clampedTime >= c.start - 0.05 && clampedTime <= c.end + 0.05) || null;
    }
    if (!clip) {
      clip = clipsList.find((c) => clampedTime >= c.start - 0.01 && clampedTime <= c.end + 0.01) || null;
    }
    if (clip) {
      const offset = Math.max(0, Math.min(clampedTime - clip.start, clip.end - clip.start));
      const srcStart = clip.sourceStart !== undefined ? clip.sourceStart : clip.start;
      return srcStart + offset;
    }
    return null;
  }

  function dispatchSeek(srcTime: number, fast = false) {
    if (!videoPlayer.value) return;
    const v = videoPlayer.value;

    if (v.seeking) {
      queuedSeekTime = srcTime;
      queuedSeekFast = fast;
      return;
    }

    queuedSeekTime = null;
    isSeeking.value = true;

    if (Math.abs(v.currentTime - srcTime) < 0.005) {
      isSeeking.value = false;
      return;
    }

    const isMatroska = currentFilePath.value.toLowerCase().endsWith('.mkv') || currentFilePath.value.toLowerCase().endsWith('.webm');
    try {
      if (fast && !isMatroska && typeof (v as any).fastSeek === 'function') {
        (v as any).fastSeek(srcTime);
      } else {
        v.currentTime = srcTime;
      }
    } catch (err) {
      console.warn('Error al ejecutar seek:', err);
      isSeeking.value = false;
    }
  }

  // VF-12: Desplaza el cabezal asociando explícitamente la instancia de clip activa
  function seekTo(
    targetTime: number,
    getClips?: () => ClipItem[],
    preferredClipId?: string | null
  ) {
    if (!videoPlayer.value || duration.value === 0) return;
    const clamped = Math.max(0, Math.min(targetTime, duration.value));
    currentTime.value = clamped;

    const clipsList = getClips ? getClips() : [];
    let clip: ClipItem | null = null;
    if (preferredClipId) {
      clip = clipsList.find((c) => c.id === preferredClipId && clamped >= c.start - 0.05 && clamped <= c.end + 0.05) || null;
    }
    if (!clip && clipsList.length > 0) {
      clip = clipsList.find((c) => clamped >= c.start - 0.01 && clamped <= c.end + 0.01) || null;
    }
    activePlaybackClipId.value = clip ? clip.id : null;

    const srcTime = resolveSourceTime(clamped, clipsList, clip?.id);

    if (srcTime !== null) {
      dispatchSeek(srcTime);
    } else {
      queuedSeekTime = null;
      if (isPlaying.value) {
        const nextClip = clipsList.find((c) => c.start >= clamped);
        if (nextClip) {
          activePlaybackClipId.value = nextClip.id;
          seekTo(nextClip.start, getClips, nextClip.id);
        } else {
          videoPlayer.value.pause();
          isPlaying.value = false;
          activePlaybackClipId.value = null;
        }
      } else {
        if (!videoPlayer.value.paused) {
          videoPlayer.value.pause();
        }
      }
    }
  }

  function onSeeked() {
    isSeeking.value = false;
    if (queuedSeekTime !== null && videoPlayer.value) {
      const nextTime = queuedSeekTime;
      const nextFast = queuedSeekFast;
      queuedSeekTime = null;
      dispatchSeek(nextTime, nextFast);
    }
  }

  function cancelPlaybackMonitoring() {
    if (playbackRafId) {
      cancelAnimationFrame(playbackRafId);
      playbackRafId = null;
    }
  }

  function scheduleNextPlaybackCheck(getClips: () => ClipItem[]) {
    if (!isPlaying.value || !videoPlayer.value) return;
    if (playbackRafId) {
      cancelAnimationFrame(playbackRafId);
    }
    playbackRafId = requestAnimationFrame(() => {
      playbackRafId = null;
      checkPlaybackBounds(getClips);
    });
  }

  function pausePlayback(getClips?: () => ClipItem[]) {
    if (!videoPlayer.value) return;
    videoPlayer.value.pause();
    isPlaying.value = false;
    cancelPlaybackMonitoring();
    stopVuMeter();

    // Sincronizar el cabezal con el punto real donde se detuvo el decodificador
    const vTime = videoPlayer.value.currentTime;
    const clipsList = getClips ? getClips() : [];
    if (clipsList.length > 0) {
      let curClip: ClipItem | null = null;
      if (activePlaybackClipId.value) {
        curClip = clipsList.find((c) => c.id === activePlaybackClipId.value) || null;
      }
      if (!curClip) {
        curClip = clipsList.find((c) => currentTime.value >= c.start - 0.05 && currentTime.value <= c.end + 0.05) || null;
      }
      if (curClip) {
        const sStart = curClip.sourceStart !== undefined ? curClip.sourceStart : curClip.start;
        const clipProgress = Math.max(0, Math.min(vTime - sStart, curClip.end - curClip.start));
        currentTime.value = curClip.start + clipProgress;
      }
    } else {
      currentTime.value = vTime;
    }

    // Cuantizar al fotograma exacto sin forzar un seek redundante
    const fps = metadata.value.fps || 30;
    currentTime.value = Math.max(0, Math.min(duration.value, Math.round(currentTime.value * fps) / fps));
  }

  function stepFrame(frameDelta: number, getClips?: () => ClipItem[]) {
    if (!videoPlayer.value) return;
    const fps = metadata.value.fps || 30;
    const frameDuration = 1 / fps;
    if (isPlaying.value) {
      pausePlayback(getClips);
    }
    // Paso de fotograma cuantizado exacto sin deriva de coma flotante
    const target = Math.round((currentTime.value + frameDelta * frameDuration) * fps) / fps;
    seekTo(target, getClips, activePlaybackClipId.value);
  }

  function stepSeconds(secondsDelta: number, getClips?: () => ClipItem[]) {
    if (!videoPlayer.value) return;
    const fps = metadata.value.fps || 30;
    if (isPlaying.value) {
      pausePlayback(getClips);
    }
    const target = Math.round((currentTime.value + secondsDelta) * fps) / fps;
    seekTo(target, getClips, activePlaybackClipId.value);
  }

  function togglePlay(getClips?: () => ClipItem[], inPt?: number, outPt?: number) {
    if (!videoPlayer.value) return;
    if (videoPlayer.value.paused) {
      if (outPt !== undefined && inPt !== undefined && outPt > inPt && currentTime.value >= outPt) {
        seekTo(inPt, getClips);
      }

      const clipsList = getClips ? getClips() : [];
      if (clipsList.length > 0) {
        let matchingClip = activePlaybackClipId.value
          ? clipsList.find((c) => c.id === activePlaybackClipId.value && currentTime.value >= c.start - 0.01 && currentTime.value < c.end - 0.001)
          : null;
        if (!matchingClip) {
          matchingClip = clipsList.find((c) => currentTime.value >= c.start - 0.001 && currentTime.value < c.end - 0.001) || null;
        }

        if (matchingClip) {
          activePlaybackClipId.value = matchingClip.id;
          // Solo si el video está lejos de la posición del cabezal (> 0.25s), sincronizarlo antes de play
          const expectedSrc = resolveSourceTime(currentTime.value, clipsList, matchingClip.id);
          if (expectedSrc !== null && Math.abs(videoPlayer.value.currentTime - expectedSrc) > 0.25) {
            videoPlayer.value.currentTime = expectedSrc;
          }
        } else {
          const nextClip = clipsList.find((c) => c.start >= currentTime.value);
          if (nextClip) {
            activePlaybackClipId.value = nextClip.id;
            seekTo(nextClip.start, getClips, nextClip.id);
          } else if (clipsList.length > 0) {
            activePlaybackClipId.value = clipsList[0].id;
            seekTo(clipsList[0].start, getClips, clipsList[0].id);
          } else {
            return;
          }
        }
      }

      videoPlayer.value.play().catch((e) => {
        if (e?.name !== 'AbortError') {
          console.warn('Error play:', e);
        }
      });
    } else {
      pausePlayback(getClips);
    }
  }

  // VF-12: Verificación de límites por instancia activa de clip en lugar de búsqueda por tiempo de fuente
  function checkPlaybackBounds(getClips: () => ClipItem[]) {
    if (!isPlaying.value || !videoPlayer.value) return;

    // Bloqueo durante seeking: no actualizar mientras el decodificador salta de un clip a otro
    if (videoPlayer.value.seeking || isSeeking.value) {
      scheduleNextPlaybackCheck(getClips);
      return;
    }

    const vTime = videoPlayer.value.currentTime;
    const clipsList = getClips();

    if (clipsList.length === 0) {
      currentTime.value = vTime;
      updateAudioLevels(vTime);
      scheduleNextPlaybackCheck(getClips);
      return;
    }

    let curClip: ClipItem | null = null;
    if (activePlaybackClipId.value) {
      curClip = clipsList.find((c) => c.id === activePlaybackClipId.value) || null;
    }

    if (!curClip) {
      curClip = clipsList.find((c) => currentTime.value >= c.start - 0.01 && currentTime.value <= c.end + 0.01) || null;
      if (curClip) {
        activePlaybackClipId.value = curClip.id;
      }
    }

    if (curClip) {
      const sStart = curClip.sourceStart !== undefined ? curClip.sourceStart : curClip.start;
      const sEnd = curClip.sourceEnd !== undefined ? curClip.sourceEnd : curClip.end;
      const clipDuration = curClip.end - curClip.start;

      // Comprobar si el decodificador de video está reproduciendo dentro del rango de esta instancia
      if (vTime >= sStart - 0.15 && vTime <= sEnd + 0.15) {
        const clipProgress = Math.max(0, Math.min(vTime - sStart, clipDuration));
        const timelinePos = curClip.start + clipProgress;
        currentTime.value = timelinePos;
        updateAudioLevels(timelinePos);

        // Margen de detección del final de clip adaptativo a los FPS (aprox 1.2 fotogramas)
        const fps = metadata.value.fps || 30;
        const endMargin = Math.max(0.02, 1.2 / fps);

        if (vTime >= sEnd - endMargin || timelinePos >= curClip.end - endMargin) {
          const curIdx = clipsList.findIndex((c) => c.id === curClip!.id);
          const nextClip = curIdx !== -1 && curIdx < clipsList.length - 1 ? clipsList[curIdx + 1] : null;

          if (nextClip) {
            activePlaybackClipId.value = nextClip.id;
            seekTo(nextClip.start, getClips, nextClip.id);
          } else {
            pausePlayback(getClips);
            activePlaybackClipId.value = null;
            return;
          }
        }
      } else {
        // Fuera del rango de fuente del clip activo: resincronizar
        const offset = Math.max(0, Math.min(currentTime.value - curClip.start, clipDuration));
        dispatchSeek(sStart + offset);
      }
    } else {
      const nextClip = clipsList.find((c) => c.start >= currentTime.value);
      if (nextClip) {
        activePlaybackClipId.value = nextClip.id;
        seekTo(nextClip.start, getClips, nextClip.id);
      } else {
        pausePlayback(getClips);
        activePlaybackClipId.value = null;
        return;
      }
    }

    scheduleNextPlaybackCheck(getClips);
  }

  function fastSeekTo(
    targetTime: number,
    getClips?: () => ClipItem[],
    preferredClipId?: string | null
  ) {
    if (!videoPlayer.value || duration.value === 0) return;
    const clamped = Math.max(0, Math.min(targetTime, duration.value));
    currentTime.value = clamped;

    const clipsList = getClips ? getClips() : [];
    let clip: ClipItem | null = null;
    if (preferredClipId) {
      clip = clipsList.find((c) => c.id === preferredClipId && clamped >= c.start - 0.05 && clamped <= c.end + 0.05) || null;
    }
    if (!clip && clipsList.length > 0) {
      clip = clipsList.find((c) => clamped >= c.start - 0.01 && clamped <= c.end + 0.01) || null;
    }
    activePlaybackClipId.value = clip ? clip.id : null;

    const srcTime = resolveSourceTime(clamped, clipsList, clip?.id);

    if (srcTime !== null) {
      dispatchSeek(srcTime, true);
    } else {
      queuedSeekTime = null;
    }
  }

  function setAudioPeaks(peaks: Uint8Array | null) {
    audioPeaksData = peaks;
  }

  function updateAudioLevels(t: number) {
    if (!isPlaying.value || !audioPeaksData || audioPeaksData.length === 0) {
      audioLevelL.value = 0;
      audioLevelR.value = 0;
      audioDbText.value = '-inf dBFS';
      return;
    }
    const idx = Math.floor(t * 100);
    const peak = audioPeaksData[idx] || 0;
    if (peak > 4) {
      const norm = peak / 255;
      const level = Math.min(100, Math.round(Math.pow(norm, 0.75) * 100));
      audioLevelL.value = level;
      audioLevelR.value = Math.max(0, Math.min(100, Math.round(level * (0.92 + (idx % 5) * 0.03))));
      const db = 20 * Math.log10(norm);
      audioDbText.value = `${db >= 0 ? '+' : ''}${db.toFixed(1)} dBFS`;
    } else {
      audioLevelL.value = 0;
      audioLevelR.value = 0;
      audioDbText.value = '-inf dBFS';
    }
  }

  function stopVuMeter() {
    audioLevelL.value = 0;
    audioLevelR.value = 0;
    audioDbText.value = '-inf dBFS';
  }

  function onPlayStarted(getClips: () => ClipItem[]) {
    isPlaying.value = true;
    cancelPlaybackMonitoring();
    updateAudioLevels(currentTime.value);
    scheduleNextPlaybackCheck(getClips);
  }

  function onPlayPaused(getClips?: () => ClipItem[]) {
    pausePlayback(getClips);
  }

  function setPlaybackRate(rate: number) {
    playbackRate.value = rate;
    if (videoPlayer.value) {
      videoPlayer.value.playbackRate = rate;
    }
  }

  function toggleMute() {
    isMuted.value = !isMuted.value;
    if (videoPlayer.value) {
      videoPlayer.value.muted = isMuted.value;
    }
  }

  function setVolume(val: number) {
    volume.value = Math.max(0, Math.min(1, val));
    if (videoPlayer.value) {
      videoPlayer.value.volume = volume.value;
      isMuted.value = volume.value === 0;
    }
  }

  function onTimeUpdate(
    getClips?: () => ClipItem[],
    isDragging?: boolean | (() => boolean)
  ) {
    // Si se está reproduciendo, el bucle de alta precisión (rVFC/RAF) es la ÚNICA fuente de verdad.
    // Ignoramos los eventos timeupdate asíncronos para evitar saltos y tirones.
    if (isPlaying.value || !videoPlayer.value || isSeeking.value || videoPlayer.value.seeking) return;

    const dragging = typeof isDragging === 'function' ? isDragging() : !!isDragging;
    if (dragging) return;

    const vTime = videoPlayer.value.currentTime;
    const clipsList = getClips ? getClips() : [];

    if (clipsList.length > 0) {
      // VF-12: Utilizar la instancia de clip activa en lugar de buscar globalmente por rango de fuente
      let clip: ClipItem | null = null;
      if (activePlaybackClipId.value) {
        clip = clipsList.find((c) => c.id === activePlaybackClipId.value) || null;
      }
      if (!clip) {
        clip = clipsList.find((c) => currentTime.value >= c.start - 0.01 && currentTime.value <= c.end + 0.01) || null;
        if (clip) {
          activePlaybackClipId.value = clip.id;
        }
      }

      if (clip) {
        const sStart = clip.sourceStart !== undefined ? clip.sourceStart : clip.start;
        const sEnd = clip.sourceEnd !== undefined ? clip.sourceEnd : clip.end;
        if (vTime >= sStart - 0.15 && vTime <= sEnd + 0.15) {
          const clipProgress = Math.max(0, Math.min(vTime - sStart, clip.end - clip.start));
          currentTime.value = clip.start + clipProgress;
        }
      }
    } else {
      currentTime.value = vTime;
    }
  }

  return {
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
    loopEnabled,
    metadata,
    isOpeningFile,
    errorMessage,
    isCodecIncompatible,
    isGeneratingPreview,
    previewProxyActive,
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
    setPlaybackRate,
    toggleMute,
    setVolume,
    onTimeUpdate,
    setAudioPeaks,
  };
}
