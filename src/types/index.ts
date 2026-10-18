export interface VideoMetadata {
  file_name: string;
  file_path: string;
  file_size: number;
  duration: number;
  width: number;
  height: number;
  fps: number;
  format: string;
  video_codec: string;
  audio_codec: string;
}

export interface SampleVideoInfo {
  name: string;
  path: string;
  size_formatted: string;
  format: string;
}

export interface ClipItem {
  id: string;
  name: string;
  start: number;
  end: number;
  sourceStart?: number;
  sourceEnd?: number;
  color: string;
}

export interface TimelineGap {
  start: number;
  end: number;
}

export interface TimelineTile {
  index: number;
  pixelLeft: number;
  width: number;
  time: number;
  image: string | null;
}

export interface RulerMark {
  time: number;
  pixel: number;
  isMajor: boolean;
  label: string | null;
}

export interface HardwareEncoderInfo {
  id: string;
  name: string;
  description: string;
  is_recommended: boolean;
}

export interface ExportProgressPayload {
  percent: number;
  stage?: string;
  fps?: number;
  speed?: string;
}

export interface ExportPlanSummary {
  plan_id?: string;
  total_duration: number;
  reencode_duration: number;
  copy_duration: number;
  segments_count: number;
  pieces_count: number;
}

export interface ExportOptionsPayload {
  input_path: string;
  output_path: string;
  plan_id?: string;
  mode: 'smart_cut' | 'reencode' | 'individual_clips';
  segments: Array<{ start: number; end: number; name?: string }>;
  hardware_accel?: string;
  video_codec?: string;
  crf?: number;
  preset?: string;
  audio_only?: boolean;
  audio_format?: string;
  container_format?: string;
}

export interface ContextMenuState {
  visible: boolean;
  x: number;
  y: number;
  clip: ClipItem | null;
}

export type ActiveTool = 'select' | 'cut';
export type AudioVisMode = 'waves' | 'spectrogram';
export type AspectRatio = '16:9' | '9:16' | '1:1' | '4:3' | '21:9';
