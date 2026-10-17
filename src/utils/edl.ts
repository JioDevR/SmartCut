import type { ClipItem } from '../types';

export function formatEdlTimecode(seconds: number, fps = 30): string {
  const safeSeconds = Math.max(0, seconds);
  const h = Math.floor(safeSeconds / 3600);
  const m = Math.floor((safeSeconds % 3600) / 60);
  const s = Math.floor(safeSeconds % 60);
  const f = Math.floor((safeSeconds % 1) * fps);

  const pad = (n: number) => String(n).padStart(2, '0');
  return `${pad(h)}:${pad(m)}:${pad(s)}:${pad(f)}`;
}

export function generateCmx3600Edl(clips: ClipItem[], fps = 30, title = 'SmartCut Timeline'): string {
  const lines: string[] = [];
  lines.push(`TITLE: ${title}`);
  lines.push('FCM: NON-DROP FRAME\n');

  let timelineRecordIn = 0;
  const sorted = [...clips].sort((a, b) => a.start - b.start);

  sorted.forEach((clip, idx) => {
    const eventNum = String(idx + 1).padStart(3, '0');
    const sourceIn = clip.sourceStart !== undefined ? clip.sourceStart : clip.start;
    const sourceOut = clip.sourceEnd !== undefined ? clip.sourceEnd : clip.end;
    const duration = Math.max(0.01, sourceOut - sourceIn);

    const srcInTc = formatEdlTimecode(sourceIn, fps);
    const srcOutTc = formatEdlTimecode(sourceOut, fps);
    const recInTc = formatEdlTimecode(timelineRecordIn, fps);
    const recOutTc = formatEdlTimecode(timelineRecordIn + duration, fps);

    // Standard CMX 3600 EDL format
    lines.push(`${eventNum}  AX       AA/V  C        ${srcInTc} ${srcOutTc} ${recInTc} ${recOutTc}`);
    lines.push(`* FROM CLIP NAME: ${clip.name}`);
    lines.push('');

    timelineRecordIn += duration;
  });

  return lines.join('\n');
}
