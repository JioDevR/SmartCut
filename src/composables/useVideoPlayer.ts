import { ref, computed, nextTick } from 'vue';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import type { VideoMetadata, SampleVideoInfo, ClipItem, AspectRatio } from '../types';

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
  const showGuides = ref(false);
  const aspectRatio = ref<AspectRatio>('16:9');

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

  const sampleVideos = ref<SampleVideoInfo[]>([]);
  const isOpeningFile = ref(false);
  const errorMessage = ref('');
  const isCodecIncompatible = ref(false);
  const isGeneratingPreview = ref(false);
  const previewProxyActive = ref(false);

  // VU Meter real con Web Audio API
  const audioLevelL = ref(0);
  const audioLevelR = ref(0);
  const audioDbText = ref('-inf dBFS');

  let audioCtx: AudioContext | null = null;
  let mediaSourceNode: MediaElementAudioSourceNode | null = null;
  let analyserL: AnalyserNode | null = null;
  let analyserR: AnalyserNode | null = null;
  let vuMeterRafId: number | null = null;
  const timeDataBufL = new Uint8Array(128);
  const timeDataBufR = new Uint8Array(128);

  let queuedSeekTime: number | null = null;
  let queuedSeekFast = false;
  let playbackRafId: number | null = null;

  async function loadSampleVideosList() {
    try {
      sampleVideos.value = await invoke<SampleVideoInfo[]>('get_sample_videos');
    } catch (err) {
      console.warn('No se pudieron obtener videos de muestra:', err);
    }
  }

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

  function resolveSourceTime(clampedTime: number, clipsList: ClipItem[]): number | null {
    if (clipsList.length === 0) return clampedTime;
    const clip = clipsList.find((c) => clampedTime >= c.start - 0.01 && clampedTime <= c.end + 0.01);
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

  function seekTo(
    targetTime: number,
    getClips?: () => ClipItem[]
  ) {
    if (!videoPlayer.value || duration.value === 0) return;
    const clamped = Math.max(0, Math.min(targetTime, duration.value));
    currentTime.value = clamped;

    const clipsList = getClips ? getClips() : [];
    const srcTime = resolveSourceTime(clamped, clipsList);

    if (srcTime !== null) {
      dispatchSeek(srcTime);
    } else {
      queuedSeekTime = null;
      if (isPlaying.value) {
        const nextClip = clipsList.find((c) => c.start >= clamped);
        if (nextClip) {
          seekTo(nextClip.start, getClips);
        } else {
          videoPlayer.value.pause();
          isPlaying.value = false;
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

  function stepFrame(frameDelta: number, getClips?: () => ClipItem[]) {
    if (!videoPlayer.value) return;
    const fps = metadata.value.fps || 30;
    const frameDuration = 1 / fps;
    videoPlayer.value.pause();
    seekTo(currentTime.value + frameDelta * frameDuration, getClips);
  }

  function stepSeconds(secondsDelta: number, getClips?: () => ClipItem[]) {
    if (!videoPlayer.value) return;
    seekTo(currentTime.value + secondsDelta, getClips);
  }

  function togglePlay(getClips?: () => ClipItem[], inPt?: number, outPt?: number) {
    if (!videoPlayer.value) return;
    if (videoPlayer.value.paused) {
      if (outPt !== undefined && inPt !== undefined && outPt > inPt && currentTime.value >= outPt) {
        seekTo(inPt, getClips);
      }

      const clipsList = getClips ? getClips() : [];
      if (clipsList.length > 0) {
        const inClip = clipsList.some((c) => currentTime.value >= c.start - 0.001 && currentTime.value < c.end - 0.001);
        if (!inClip) {
          const nextClip = clipsList.find((c) => c.start >= currentTime.value);
          if (nextClip) {
            seekTo(nextClip.start, getClips);
          } else if (clipsList.length > 0) {
            seekTo(clipsList[0].start, getClips);
          } else {
            return;
          }
        }
      }

      videoPlayer.value.play().catch((e) => console.warn('Error play:', e));
    } else {
      videoPlayer.value.pause();
    }
  }

  function checkPlaybackBounds(getClips: () => ClipItem[]) {
    if (!isPlaying.value || !videoPlayer.value) return;

    const vTime = videoPlayer.value.currentTime;
    const clipsList = getClips();

    const curClip = clipsList.find((c) => {
      const sStart = c.sourceStart !== undefined ? c.sourceStart : c.start;
      const sEnd = c.sourceEnd !== undefined ? c.sourceEnd : c.end;
      return vTime >= sStart - 0.05 && vTime <= sEnd + 0.05;
    });

    if (curClip) {
      const sStart = curClip.sourceStart !== undefined ? curClip.sourceStart : curClip.start;
      const sEnd = curClip.sourceEnd !== undefined ? curClip.sourceEnd : curClip.end;
      const timelinePos = curClip.start + Math.max(0, vTime - sStart);
      currentTime.value = timelinePos;

      if (vTime >= sEnd - 0.035 || timelinePos >= curClip.end - 0.035) {
        const curIdx = clipsList.findIndex((c) => c.id === curClip.id);
        const nextClip = curIdx !== -1 && curIdx < clipsList.length - 1 ? clipsList[curIdx + 1] : null;

        if (nextClip) {
          seekTo(nextClip.start, getClips);
        } else {
          videoPlayer.value.pause();
          isPlaying.value = false;
          currentTime.value = curClip.end;
          return;
        }
      }
    } else {
      const nextClip = clipsList.find((c) => c.start >= currentTime.value);
      if (nextClip) {
        seekTo(nextClip.start, getClips);
      } else {
        videoPlayer.value.pause();
        isPlaying.value = false;
        return;
      }
    }

    if (isPlaying.value) {
      playbackRafId = requestAnimationFrame(() => checkPlaybackBounds(getClips));
    }
  }

  function fastSeekTo(
    targetTime: number,
    getClips?: () => ClipItem[]
  ) {
    if (!videoPlayer.value || duration.value === 0) return;
    const clamped = Math.max(0, Math.min(targetTime, duration.value));
    currentTime.value = clamped;

    const clipsList = getClips ? getClips() : [];
    const srcTime = resolveSourceTime(clamped, clipsList);

    if (srcTime !== null) {
      dispatchSeek(srcTime, true);
    } else {
      queuedSeekTime = null;
    }
  }

  function initAudioAnalyser() {
    if (!videoPlayer.value || audioCtx) return;
    try {
      const AudioCtxClass = window.AudioContext || (window as any).webkitAudioContext;
      if (!AudioCtxClass) return;
      audioCtx = new AudioCtxClass();
      mediaSourceNode = audioCtx.createMediaElementSource(videoPlayer.value);

      analyserL = audioCtx.createAnalyser();
      analyserR = audioCtx.createAnalyser();
      analyserL.fftSize = 256;
      analyserR.fftSize = 256;

      const splitter = audioCtx.createChannelSplitter(2);
      const merger = audioCtx.createChannelMerger(2);

      mediaSourceNode.connect(splitter);

      splitter.connect(analyserL, 0);
      analyserL.connect(merger, 0, 0);

      splitter.connect(analyserR, splitter.numberOfOutputs > 1 ? 1 : 0);
      analyserR.connect(merger, 0, 1);

      merger.connect(audioCtx.destination);
    } catch (e) {
      console.warn('Web Audio Analyser no pudo inicializarse:', e);
    }
  }

  function startVuMeter() {
    if (!audioCtx && videoPlayer.value) {
      initAudioAnalyser();
    }
    if (audioCtx && audioCtx.state === 'suspended') {
      audioCtx.resume().catch(() => {});
    }

    if (vuMeterRafId) cancelAnimationFrame(vuMeterRafId);

    const updateVu = () => {
      if (!isPlaying.value) {
        audioLevelL.value = 0;
        audioLevelR.value = 0;
        audioDbText.value = '-inf dBFS';
        return;
      }

      if (analyserL && analyserR) {
        analyserL.getByteTimeDomainData(timeDataBufL);
        analyserR.getByteTimeDomainData(timeDataBufR);

        let sumL = 0;
        for (let i = 0; i < timeDataBufL.length; i++) {
          const val = (timeDataBufL[i] - 128) / 128;
          sumL += val * val;
        }
        const rmsL = Math.sqrt(sumL / timeDataBufL.length);

        let sumR = 0;
        for (let i = 0; i < timeDataBufR.length; i++) {
          const val = (timeDataBufR[i] - 128) / 128;
          sumR += val * val;
        }
        const rmsR = Math.sqrt(sumR / timeDataBufR.length);

        const maxRms = Math.max(rmsL, rmsR);
        if (maxRms < 0.005) {
          audioLevelL.value = 0;
          audioLevelR.value = 0;
          audioDbText.value = '-inf dBFS';
        } else {
          const db = 20 * Math.log10(maxRms);
          audioDbText.value = `${db >= 0 ? '+' : ''}${db.toFixed(1)} dBFS`;
          audioLevelL.value = Math.min(100, Math.round(Math.pow(Math.min(1, rmsL * 2.2), 0.6) * 100));
          audioLevelR.value = Math.min(100, Math.round(Math.pow(Math.min(1, rmsR * 2.2), 0.6) * 100));
        }
      }

      vuMeterRafId = requestAnimationFrame(updateVu);
    };

    vuMeterRafId = requestAnimationFrame(updateVu);
  }

  function stopVuMeter() {
    if (vuMeterRafId) {
      cancelAnimationFrame(vuMeterRafId);
      vuMeterRafId = null;
    }
    audioLevelL.value = 0;
    audioLevelR.value = 0;
    audioDbText.value = '-inf dBFS';
  }

  function onPlayStarted(getClips: () => ClipItem[]) {
    isPlaying.value = true;
    startVuMeter();
    if (playbackRafId) cancelAnimationFrame(playbackRafId);
    playbackRafId = requestAnimationFrame(() => checkPlaybackBounds(getClips));
  }

  function onPlayPaused() {
    isPlaying.value = false;
    stopVuMeter();
    if (playbackRafId) {
      cancelAnimationFrame(playbackRafId);
      playbackRafId = null;
    }
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
    inPt?: number,
    outPt?: number,
    isDragging?: boolean | (() => boolean)
  ) {
    if (!videoPlayer.value || isSeeking.value || videoPlayer.value.seeking) return;

    // Problema 1: Evitar que onTimeUpdate pise el cabezal mientras se arrastra
    // o cuando el video está pausado (el seek asíncrono emite timeupdate desfasado del decodificador).
    const dragging = typeof isDragging === 'function' ? isDragging() : !!isDragging;
    if (!isPlaying.value || videoPlayer.value.paused || dragging) return;

    const vTime = videoPlayer.value.currentTime;
    const clipsList = getClips ? getClips() : [];

    if (clipsList.length > 0) {
      const clip = clipsList.find((c) => {
        const sStart = c.sourceStart !== undefined ? c.sourceStart : c.start;
        const sEnd = c.sourceEnd !== undefined ? c.sourceEnd : c.end;
        return vTime >= sStart && vTime < sEnd;
      });
      if (clip) {
        const sStart = clip.sourceStart !== undefined ? clip.sourceStart : clip.start;
        currentTime.value = clip.start + Math.max(0, vTime - sStart);
      }
    } else {
      currentTime.value = vTime;
    }

    if (loopEnabled.value && outPt !== undefined && inPt !== undefined && outPt > inPt && currentTime.value >= outPt) {
      seekTo(inPt, getClips);
    }
  }

  return {
    videoPlayer,
    currentFilePath,
    videoSrc,
    isPlaying,
    isSeeking,
    isBuffering,
    currentTime,
    duration,
    volume,
    isMuted,
    playbackRate,
    loopEnabled,
    showGuides,
    aspectRatio,
    metadata,
    sampleVideos,
    isOpeningFile,
    errorMessage,
    isCodecIncompatible,
    isGeneratingPreview,
    previewProxyActive,
    audioLevelL,
    audioLevelR,
    audioDbText,
    loadSampleVideosList,
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
  };
}
