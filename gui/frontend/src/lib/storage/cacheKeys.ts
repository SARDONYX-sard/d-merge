import { OBJECT } from '@/lib/object-utils';

const CONVERT_PUB_CACHE_KEYS_OBJ = {
  convertSelectionType: 'convert-selection-type',
  convertSelectedFiles: 'convert-selected-files',
  convertSelectedDirs: 'convert-selected-dirs',
  convertSelectedTree: 'convert-selected-tree',
  convertOutput: 'convert-output',
  convertOutFmt: 'convert-output-fmt',
} as const;

const PUB_CACHE_KEYS_OBJ = {
  customCss: 'custom-css',
  customJs: 'custom-js',
  customTranslationDict: 'custom-translation-dict',
  editorMode: 'editor-mode',
  editorTabSelect: 'editor-tab-select',
  lastPath: 'last-path', // last visited url(in App)
  locale: 'locale',
  logLevel: 'log-level',
  presetNumber: 'css-preset-number',
  selectedPage: 'selected-page',
  settingsTabSelect: 'settings-tab-select',
  settingsTabPosition: 'settings-tab-position',
  snackbarLimit: 'snackbar-limit',
  snackbarPosition: 'snackbar-position',
} as const;

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

const PATCH_PRIVATE_CACHE_KEYS_OBJ = {
  patchInput: 'patch-input',
  patchOutput: 'patch-output',
} as const;

const PRIVATE_CACHE_KEYS_OBJ = {
  exportSettingsPath: 'export-settings-path',
  importSettingsPath: 'import-backup-path',
  langFilePath: 'lang-file-path',
} as const;

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

export const HIDDEN_CACHE_OBJ = {
  runScript: 'run-script',
} as const;

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

export const PUB_CACHE_OBJ = {
  ...CONVERT_PUB_CACHE_KEYS_OBJ,
  ...PUB_CACHE_KEYS_OBJ,
  ...CONVERT_PUB_CACHE_KEYS_OBJ,
} as const;

export const PRIVATE_CACHE_OBJ = {
  ...PATCH_PRIVATE_CACHE_KEYS_OBJ,
  ...PRIVATE_CACHE_KEYS_OBJ,
} as const;

/** Public cache keys that are available and exposed for standard use in the application. */
export const PUB_CACHE_KEYS = [...OBJECT.values(PUB_CACHE_OBJ)] as const;

/** Private cache keys that are internal to the application and may involve sensitive data or paths. */
const PRIVATE_CACHE_KEYS = [...OBJECT.values(PRIVATE_CACHE_OBJ)] as const;

/** Hidden cache keys, typically used for restricted data like permissions for running scripts. */
export const HIDDEN_CACHE_KEYS = [...OBJECT.values(HIDDEN_CACHE_OBJ)] as const;

/** Aggregated list of both public and private cache keys. */
export const CACHE_KEYS = [...PUB_CACHE_KEYS, ...PRIVATE_CACHE_KEYS] as const;
