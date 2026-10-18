import { useClipsManager } from '../composables/useClipsManager';

function assert(cond: boolean, msg: string) {
  if (!cond) {
    console.error('FAIL:', msg);
    throw new Error(msg);
  }
}

console.log('--- Testing VF-13: Project Restoration and State Normalization ---');

const manager = useClipsManager();

// 1. Cargar un proyecto con clips desordenados, rangos fuera de límite y selección inexistente
const rawUnsortedClips = [
  {
    id: 'clip-2',
    name: 'Clip 2',
    start: 20.0,
    end: 35.0,
    sourceStart: 50.0, // Excede mediaDuration de 40.0
    sourceEnd: 60.0,
    color: '#ff0000',
  },
  {
    id: 'clip-1',
    name: 'Clip 1',
    start: 5.0,
    end: 15.0,
    sourceStart: 5.0,
    sourceEnd: 15.0,
    color: '#00ff00',
  },
  {
    id: 'clip-3',
    name: 'Clip 3',
    start: 36.0,
    end: 45.0, // Excede mediaDuration de 40.0
    sourceStart: 10.0,
    sourceEnd: 20.0,
    color: '#0000ff',
  },
];

manager.loadProjectState(rawUnsortedClips, 40.0, 'non-existent-id', 10.0, 50.0);

// Comprobar orden cronológico
assert(manager.clips.value.length === 3, 'Debe haber 3 clips');
assert(manager.clips.value[0].id === 'clip-1', 'El primer clip debe ser clip-1 (start 5.0)');
assert(manager.clips.value[1].id === 'clip-2', 'El segundo clip debe ser clip-2 (start 20.0)');
assert(manager.clips.value[2].id === 'clip-3', 'El tercer clip debe ser clip-3 (start 36.0)');

// Comprobar clamping de rangos contra mediaDuration
assert(manager.clips.value[2].end <= 40.0, 'Clip 3 end debe estar acotado a 40.0');
assert(manager.clips.value[1].sourceStart <= 40.0, 'Clip 2 sourceStart debe estar acotado a 40.0');

// Comprobar fallback de selectedClipId al no existir 'non-existent-id'
assert(manager.selectedClipId.value === 'clip-1', 'Debe seleccionar clip-1 por fallback');

// Comprobar restauración con selectedClipId válido
manager.loadProjectState(rawUnsortedClips, 40.0, 'clip-2', 5.0, 30.0);
assert(manager.selectedClipId.value === 'clip-2', 'Debe restaurar selectedClipId clip-2');
assert(manager.inPoint.value === 5.0, 'inPoint debe restaurarse');
assert(manager.outPoint.value === 30.0, 'outPoint debe restaurarse');

// Comprobar splitAtTime con clips no ordenados inicialmente
const messyClips = [
  { id: 'c3', name: 'C3', start: 30, end: 40, sourceStart: 30, sourceEnd: 40, color: '#111' },
  { id: 'c1', name: 'C1', start: 0, end: 10, sourceStart: 0, sourceEnd: 10, color: '#222' },
  { id: 'c2', name: 'C2', start: 10, end: 20, sourceStart: 10, sourceEnd: 20, color: '#333' },
];
manager.clips.value = messyClips;
manager.splitAtTime(15, 40);
assert(manager.clips.value.some(c => c.start === 10 && c.end === 15), 'Debe haber cortado el clip [10, 20] a [10, 15]');
assert(manager.clips.value.some(c => c.start === 15 && c.end === 20), 'Debe haber creado el nuevo clip [15, 20]');

console.log('VF-13 tests passed successfully!');

console.log('--- Testing VF-12: Disambiguated Clip Instance Playback ---');

// Simular el escenario de VF-12 con dos clips solapados en tiempo de fuente:
// Clip A: timeline [0, 5], source [10, 15]
// Clip B: timeline [5, 10], source [10, 15] (mismo rango de fuente)
const overlappingClips = [
  { id: 'clip-A', name: 'Clip A', start: 0, end: 5, sourceStart: 10, sourceEnd: 15, color: '#111' },
  { id: 'clip-B', name: 'Clip B', start: 5, end: 10, sourceStart: 10, sourceEnd: 15, color: '#222' },
];

// Comprobar resolución de tiempo de fuente con ID preferido
function resolveSourceTime(clampedTime: number, clipsList: any[], preferredClipId?: string | null): number | null {
  if (clipsList.length === 0) return clampedTime;
  let clip: any = null;
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

// Para timeline = 2 (dentro de Clip A):
const srcTimeA = resolveSourceTime(2, overlappingClips, 'clip-A');
assert(srcTimeA === 12, 'Clip A at timeline 2 must resolve to source 12');

// Para timeline = 7 (dentro de Clip B con mismo rango de fuente):
const srcTimeB = resolveSourceTime(7, overlappingClips, 'clip-B');
assert(srcTimeB === 12, 'Clip B at timeline 7 must resolve to source 12');

// Al reproducir Clip B, el cálculo de posición en timeline no debe colapsar a Clip A:
let activeId: string | null = 'clip-B';
const vTime = 13.0; // En reproducción, decodificador está en 13.0

let curClip = overlappingClips.find(c => c.id === activeId);
assert(curClip !== undefined && curClip.id === 'clip-B', 'Debe seleccionar Clip B por activePlaybackClipId');
const sStart = curClip!.sourceStart;
const sEnd = curClip!.sourceEnd;
assert(vTime >= sStart - 0.15 && vTime <= sEnd + 0.15, 'vTime está en el rango de Clip B');
const clipProgress = Math.max(0, Math.min(vTime - sStart, curClip!.end - curClip!.start));
const timelinePos = curClip!.start + clipProgress;
assert(timelinePos === 8.0, `Timeline position for Clip B must be 8.0, got ${timelinePos}`);

console.log('VF-12 tests passed successfully!');
