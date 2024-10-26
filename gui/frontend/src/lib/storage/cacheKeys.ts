import { OBJECT } from '@/lib/object-utils';

const CONVERT_PUB_CACHE_KEYS_OBJ = {
  convertSelectionType: 'convert-selection-type',
  convertSelectedFiles: 'convert-selected-files',
  convertSelectedDirs: 'convert-selected-dirs',
  convertOutput: 'convert-output',
  convertOutFmt: 'convert-output-fmt',
} as const;

const PUB_CACHE_KEYS_OBJ = {
  selectedPage: 'selected-page',
  customCss: 'custom-css',
  presetNumber: 'css-preset-number',
  editorMode: 'editor-mode',
  customJs: 'custom-js',
  logLevel: 'log-level',
  customTranslationDict: 'custom-translation-dict',
  editorTabSelect: 'editor-tab-select',
  locale: 'locale',
  settingsTabSelect: 'settings-tab-select',
  snackbarLimit: 'snackbar-limit',
  snackbarPosition: 'snackbar-position',
} as const;

const PRIVATE_CACHE_KEYS_OBJ = {
  exportSettingsPath: 'export-settings-path',
  importSettingsPath: 'import-backup-path',
  langFilePath: 'lang-file-path',
} as const;

export const PUB_CACHE_OBJ = {
  ...CONVERT_PUB_CACHE_KEYS_OBJ,
  ...PUB_CACHE_KEYS_OBJ,
  ...CONVERT_PUB_CACHE_KEYS_OBJ,
} as const;

export const PRIVATE_CACHE_OBJ = {
  ...PRIVATE_CACHE_KEYS_OBJ,
} as const;

/** Public cache keys that are available and exposed for standard use in the application. */
export const PUB_CACHE_KEYS = [...OBJECT.values(PUB_CACHE_OBJ)] as const;

/** Private cache keys that are internal to the application and may involve sensitive data or paths. */
const PRIVATE_CACHE_KEYS = [...OBJECT.values(PRIVATE_CACHE_OBJ)] as const;

/** Hidden cache keys, typically used for restricted data like permissions for running scripts. */
export const HIDDEN_CACHE_KEYS = ['run-script'] as const;

/** Aggregated list of both public and private cache keys. */
export const CACHE_KEYS = [...PUB_CACHE_KEYS, ...PRIVATE_CACHE_KEYS] as const;
