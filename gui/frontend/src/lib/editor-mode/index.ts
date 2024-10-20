import { STORAGE } from '@/lib/storage';

export type EditorMode = 'default' | 'vim';

const DEFAULT = 'default';
const CACHE_KEY = 'editor-mode';

/**
 * `'error'` if null or undefined
 * @default `error`
 */
const normalize = (mode?: string | null): EditorMode => {
  if (mode === 'vim') {
    return mode;
  }
  return DEFAULT;
};

export const EDITOR_MODE = {
  default: DEFAULT,

  /** Fallback to `'default'` if `null` or `undefined`. */
  normalize,

  /** get current editor code from `LocalStorage`. */
  get() {
    return normalize(STORAGE.get(CACHE_KEY));
  },

  /** set editor mode to `LocalStorage`. */
  set(level: EditorMode) {
    STORAGE.set(CACHE_KEY, level);
  },
};
