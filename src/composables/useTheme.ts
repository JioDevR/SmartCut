import { ref } from 'vue';

export interface ThemeOption {
  id: string;
  name: string;
  type: 'dark' | 'light';
  previewAccent: string;
  previewBg: string;
  previewSurface: string;
  description: string;
}

export const AVAILABLE_THEMES: ThemeOption[] = [
  {
    id: 'obsidian',
    name: 'Obsidian Precision',
    type: 'dark',
    previewAccent: '#06b6d4',
    previewBg: '#0b0f17',
    previewSurface: '#131b2a',
    description: 'Estudio oscuro con acento cian eléctrico quirúrgico.',
  },
  {
    id: 'midnight',
    name: 'Midnight Indigo',
    type: 'dark',
    previewAccent: '#818cf8',
    previewBg: '#080c18',
    previewSurface: '#0e1424',
    description: 'Noche profunda con tonos índigo y violeta neón.',
  },
  {
    id: 'emerald',
    name: 'Emerald Matrix',
    type: 'dark',
    previewAccent: '#10b981',
    previewBg: '#090d0b',
    previewSurface: '#0f1713',
    description: 'Grafito oscuro con verde esmeralda técnico y fresco.',
  },
  {
    id: 'amber',
    name: 'Amber Cinema',
    type: 'dark',
    previewAccent: '#f59e0b',
    previewBg: '#120f0d',
    previewSurface: '#1c1714',
    description: 'Carbón cálido cinematográfico con destellos ámbar y dorado.',
  },
  {
    id: 'nordic',
    name: 'Nordic Frost',
    type: 'dark',
    previewAccent: '#38bdf8',
    previewBg: '#0a111a',
    previewSurface: '#101a26',
    description: 'Azul polar ártico con reflejos celestes y hielo.',
  },
  {
    id: 'light',
    name: 'Studio Light (Claro)',
    type: 'light',
    previewAccent: '#0284c7',
    previewBg: '#f8fafc',
    previewSurface: '#f1f5f9',
    description: 'Lienzo diurno limpio de alta luminosidad y contraste nítido.',
  },
];

const currentThemeId = ref<string>('obsidian');

export function useTheme() {
  function applyTheme(themeId: string) {
    const validTheme = AVAILABLE_THEMES.find((t) => t.id === themeId);
    const idToApply = validTheme ? validTheme.id : 'obsidian';

    currentThemeId.value = idToApply;
    document.documentElement.setAttribute('data-theme', idToApply);
    try {
      localStorage.setItem('smartcut-theme', idToApply);
    } catch (_) {}
  }

  function initTheme() {
    let saved = 'obsidian';
    try {
      saved = localStorage.getItem('smartcut-theme') || 'obsidian';
    } catch (_) {}
    applyTheme(saved);
  }

  return {
    currentThemeId,
    themes: AVAILABLE_THEMES,
    applyTheme,
    initTheme,
  };
}
