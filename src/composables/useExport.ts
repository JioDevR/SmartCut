import { ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener';
import type { HardwareEncoderInfo, ClipItem, VideoMetadata, ExportProgressPayload, ExportPlanSummary } from '../types';

export function useExport() {
  const isExportModalOpen = ref(false);
  const exportOutputPath = ref('');
  const exportMode = ref<'smart_cut' | 'reencode' | 'individual_clips'>('smart_cut');
  const availableEncoders = ref<HardwareEncoderInfo[]>([]);
  const selectedEncoder = ref('nvenc');
  const selectedVideoCodec = ref('h264');
  const isExporting = ref(false);
  const exportProgressPercent = ref(0);
  const exportStageText = ref('');
  const exportFps = ref(0);
  const exportSpeed = ref('');
  const exportFinishedSuccess = ref(false);
  const exportErrorMessage = ref('');

  // Opciones avanzadas de exportación: MKV por defecto por máxima robustez en streaming/scrubbing
  const containerFormat = ref<'mp4' | 'mkv' | 'mov' | 'webm'>('mkv');
  const crf = ref<number>(20);
  const preset = ref<'ultrafast' | 'fast' | 'medium' | 'slow'>('fast');
  const audioOnly = ref<boolean>(false);
  const audioFormat = ref<'mp3' | 'aac' | 'flac'>('mp3');

  function updateOutputExtension() {
    if (!exportOutputPath.value) return;
    const lastDot = exportOutputPath.value.lastIndexOf('.');
    const ext = audioOnly.value ? audioFormat.value : containerFormat.value;
    if (lastDot !== -1) {
      exportOutputPath.value = `${exportOutputPath.value.substring(0, lastDot)}.${ext}`;
    } else {
      exportOutputPath.value = `${exportOutputPath.value}.${ext}`;
    }
  }

  let unlistenExportProgress: UnlistenFn | null = null;

  const planSummary = ref<ExportPlanSummary | null>(null);

  function getFileExtension(path: string): string {
    const lastDot = path.lastIndexOf('.');
    if (lastDot !== -1) {
      return path.substring(lastDot + 1).toLowerCase();
    }
    return 'mp4';
  }

  const exportScope = ref<'selected' | 'timeline'>('selected');
  const cachedCurrentFilePath = ref('');
  const cachedClips = ref<ClipItem[]>([]);
  const cachedSelectedClip = ref<ClipItem | null>(null);

  function getActiveSegments(): Array<{ start: number; end: number; name?: string }> {
    if (exportScope.value === 'selected' && cachedSelectedClip.value) {
      const c = cachedSelectedClip.value;
      return [
        {
          start: c.sourceStart !== undefined ? c.sourceStart : c.start,
          end: c.sourceEnd !== undefined ? c.sourceEnd : c.end,
          name: c.name,
        },
      ];
    }
    return cachedClips.value.map((c) => ({
      start: c.sourceStart !== undefined ? c.sourceStart : c.start,
      end: c.sourceEnd !== undefined ? c.sourceEnd : c.end,
      name: c.name,
    }));
  }

  function refreshPlanPreview() {
    if (!cachedCurrentFilePath.value || !exportOutputPath.value) return;
    const segments = getActiveSegments();
    if (segments.length === 0) return;

    invoke<ExportPlanSummary>('get_export_plan_preview', {
      options: {
        input_path: cachedCurrentFilePath.value,
        output_path: exportOutputPath.value,
        segments,
        mode: exportMode.value,
        hardware_accel: selectedEncoder.value,
        video_codec: selectedVideoCodec.value,
        crf: crf.value,
        preset: preset.value,
        audio_only: audioOnly.value,
        audio_format: audioFormat.value,
        container_format: containerFormat.value,
      },
    })
      .then((summary) => {
        planSummary.value = summary;
        exportErrorMessage.value = '';
      })
      .catch((err) => {
        console.warn('Error obteniendo vista previa del plan:', err);
        exportErrorMessage.value = typeof err === 'string' ? err : (err?.message || 'Error en vista previa del plan');
      });
  }

  watch([exportMode, containerFormat, audioOnly, audioFormat, crf, preset, selectedEncoder, selectedVideoCodec], () => {
    updateOutputExtension();
    refreshPlanPreview();
  });

  function setExportScope(scope: 'selected' | 'timeline') {
    exportScope.value = scope;
    refreshPlanPreview();
  }

  async function openExportModal(
    currentFilePath: string,
    metadata: VideoMetadata,
    sortedClips?: ClipItem[],
    selectedClip?: ClipItem | null
  ) {
    if (!currentFilePath) return;

    cachedCurrentFilePath.value = currentFilePath;
    cachedClips.value = sortedClips ? [...sortedClips] : [];
    cachedSelectedClip.value = selectedClip || null;

    // Si hay más de un clip y hay uno seleccionado, sugerir por defecto exportar solo el clip seleccionado
    if (selectedClip && sortedClips && sortedClips.length > 1) {
      exportScope.value = 'selected';
    } else {
      exportScope.value = 'timeline';
    }

    const inputExt = getFileExtension(currentFilePath);
    const lastSlash = Math.max(currentFilePath.lastIndexOf('/'), currentFilePath.lastIndexOf('\\'));
    const dir = lastSlash !== -1 ? currentFilePath.substring(0, lastSlash) : '';
    const sep = currentFilePath.includes('\\') ? '\\' : '/';
    const baseName = metadata.file_name.substring(0, metadata.file_name.lastIndexOf('.')) || 'video';
    
    exportOutputPath.value = dir ? `${dir}${sep}${baseName}_smartcut.${inputExt}` : `${baseName}_smartcut.${inputExt}`;

    isExportModalOpen.value = true;
    exportFinishedSuccess.value = false;
    exportErrorMessage.value = '';
    exportProgressPercent.value = 0;
    exportStageText.value = '';
    planSummary.value = null;

    refreshPlanPreview();

    try {
      const encs = await invoke<HardwareEncoderInfo[]>('get_available_encoders');
      availableEncoders.value = encs;
      if (encs.length > 0) {
        const rec = encs.find((e) => e.is_recommended);
        selectedEncoder.value = rec ? rec.id : encs[0].id;
      }
    } catch (err) {
      console.warn('Error cargando encoders disponibles:', err);
    }
  }

  async function browseExportSavePath() {
    try {
      const lastDot = exportOutputPath.value.lastIndexOf('.');
      const ext = lastDot !== -1 ? exportOutputPath.value.substring(lastDot + 1).toLowerCase() : 'mp4';
      const defaultName = exportOutputPath.value.split(/[/|\\]/).pop() || `video_smartcut.${ext}`;
      const chosen = await invoke<string | null>('pick_save_path', {
        defaultName,
        defaultExt: ext,
      });
      if (chosen) {
        exportOutputPath.value = chosen;
        refreshPlanPreview();
      }
    } catch (err) {
      console.error('Error al seleccionar ruta de guardado:', err);
    }
  }

  async function startExport(currentFilePath: string, sortedClips: ClipItem[], videoCodec: string, selectedClip?: ClipItem | null) {
    if (!exportOutputPath.value) {
      await browseExportSavePath();
      if (!exportOutputPath.value) return;
    }

    cachedCurrentFilePath.value = currentFilePath;
    if (sortedClips) cachedClips.value = [...sortedClips];
    if (selectedClip !== undefined) cachedSelectedClip.value = selectedClip;

    isExporting.value = true;
    exportProgressPercent.value = 0;
    exportStageText.value = 'Iniciando motor de renderizado acelerado...';
    exportFinishedSuccess.value = false;
    exportErrorMessage.value = '';

    if (unlistenExportProgress) {
      unlistenExportProgress();
      unlistenExportProgress = null;
    }

    unlistenExportProgress = await listen<ExportProgressPayload>('export-progress', (event) => {
      const data = event.payload;
      if (data) {
        exportProgressPercent.value = Math.round(data.percent);
        exportStageText.value = data.stage || '';
        exportFps.value = data.fps || 0;
        exportSpeed.value = data.speed || '';
      }
    });

    const segments = getActiveSegments();

    try {
      await invoke('export_video', {
        options: {
          input_path: currentFilePath,
          output_path: exportOutputPath.value,
          plan_id: planSummary.value?.plan_id,
          mode: exportMode.value,
          segments,
          hardware_accel: selectedEncoder.value,
          video_codec: videoCodec,
          crf: crf.value,
          preset: preset.value,
          audio_only: audioOnly.value,
          audio_format: audioFormat.value,
          container_format: containerFormat.value,
        },
      });
      exportProgressPercent.value = 100;
      exportFinishedSuccess.value = true;
      exportStageText.value = '¡Exportación finalizada con éxito!';
    } catch (err: any) {
      console.error('Error exportando video:', err);
      exportErrorMessage.value = typeof err === 'string' ? err : (err?.message || 'Error durante la exportación');
    } finally {
      isExporting.value = false;
      if (unlistenExportProgress) {
        unlistenExportProgress();
        unlistenExportProgress = null;
      }
    }
  }

  async function cancelExport() {
    try {
      await invoke('cancel_export');
      exportStageText.value = 'Cancelando proceso...';
      isExporting.value = false;
    } catch (err) {
      console.error('Error cancelando exportación:', err);
    }
  }

  async function openExportedFolder() {
    if (exportOutputPath.value) {
      try {
        await revealItemInDir(exportOutputPath.value);
      } catch (_) {
        try {
          const lastSlash = Math.max(exportOutputPath.value.lastIndexOf('/'), exportOutputPath.value.lastIndexOf('\\'));
          const dir = lastSlash !== -1 ? exportOutputPath.value.substring(0, lastSlash) : '';
          if (dir) {
            await openPath(dir);
          }
        } catch (e) {
          console.warn('No se pudo abrir carpeta:', e);
        }
      }
    }
  }

  async function openExportedFile() {
    if (exportOutputPath.value) {
      try {
        await openPath(exportOutputPath.value);
      } catch (e) {
        console.warn('No se pudo abrir archivo:', e);
      }
    }
  }

  function cleanupExportListener() {
    if (unlistenExportProgress) {
      unlistenExportProgress();
      unlistenExportProgress = null;
    }
  }

  return {
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
  };
}
