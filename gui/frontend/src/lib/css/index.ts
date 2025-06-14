import { STORAGE } from '@/lib/storage';
import { PUB_CACHE_OBJ } from '@/lib/storage/cacheKeys';

/**
 * Creates a CSS preset.
 *
 * @template T - The type of the CSS string.
 * @param css - The CSS string.
 * @returns - The CSS preset as a readonly string literal type.
 */
const createPreset = <T extends string>(css: T) => {
  return /* css */ `:root {
  ${css.trim()}
}

.MuiDataGrid-container--top,
.MuiDataGrid-row--borderBottom {
  background-color: none;
}
.MuiDataGrid-columnHeader,
.MuiDataGrid-columnHeaders {
  background-color: rgba(7, 7, 7, 0.77) !important;
}

.MuiDataGrid-row.Mui-selected {
  background-color: rgba(81, 81, 81, 0.21) !important;
}

body {
  background-attachment: fixed;
  background-color: #000;
  background-image: var(--image-url);
  background-position-x: var(--image-position-x);
  background-position-y: var(--image-position-y);
  background-repeat: no-repeat;
  background-size: var(--image-size);
}

main {
  background-color: var(--main-bg-color);
}

:-webkit-autofill {
  box-shadow: var(--autofill-color) 0px 0px 0px 100px inset !important;
}

.monaco-editor,
.monaco-editor-background {
  background-color: #2121213b !important;
  --vscode-editorGutter-background: #283d671a !important;
}

.decorationsOverviewRuler,
.monaco-editor .minimap canvas {
  opacity: 0.6;
}

.MuiOutlinedInput-root {
  background-color: #2424248c !important;
}

p.Mui-error {
  background-color: #2f2e2eba;
  color: var(--mui-palette-error-main);
}

.MuiIconButton-colorPrimary,
.Mui-checked,
.Mui-selected,
.MuiButton-outlined,
.MuiButton-root.MuiButton-text,
.MuiCircularProgress-svg,
.MuiInputBase-root.MuiOutlinedInput-root.MuiInputBase-colorPrimary.Mui-focused>fieldset,
[class$="MuiFormLabel-root"].Mui-focused,
[class$="MuiInputBase-root-MuiInput-root"]::after,
a,
input[aria-label="Select all rows"] + svg,
label.Mui-focused,
span.Mui-checked>svg {
  color: var(--mui-palette-primary-main) !important;
  border-color: var(--mui-palette-primary-main);
}

.MuiButton-outlined {
  background-color: #2424248c;
}

.MuiButton-outlined:hover {
  color: #fff !important;
  background-color: var(--hover-btn-color);
}

.MuiButton-contained,
#x-data-grid-selected {
  color: #fff;
  background-color: var(--convert-btn-color);
}

.MuiButton-contained:hover {
  background-color: var(--hover-convert-btn-color);
}

.MuiLinearProgress-bar,
.MuiTabs-indicator {
  background-color: var(--mui-palette-primary-main);
}
` as const;
};

const preset1 = createPreset(`
  --mui-palette-primary-main: #ff8e16;
  --mui-palette-error-main: #ff1655;
  --mui-palette-background-default: #2223;
  --mui-palette-secondary-main: #ab2b7e6e;
  --mui-filled-input-background: #2424248c;
  --mui-focus-visible: #c9623db3;
  --mui-palette-primary-dark: #cd2c6c95;

  --autofill-color: #691c3747;
  --convert-btn-color: #ab2b7e6e;
  --hover-btn-color: #c9623db3;
  --hover-convert-btn-color: #cd2c6c95;
  --image-size: cover;
  --image-url: url("https://i.redd.it/red-forest-1920-1080-v0-s9u8ki2rr70a1.jpg?s=139edf608c428656505a143635a0687dec086229");
  --main-bg-color: #2223;
`);

const preset2 = createPreset(`
  --mui-palette-primary-main: #5a9ab9;
  --mui-palette-background-default: #2226;
  --mui-palette-secondary-main: #0288d162;
  --mui-filled-input-background: #26323866;
  --mui-focus-visible: #01579b80;
  --mui-palette-primary-dark: #1c2c3a;

  --autofill-color: #5eb1ef24;
  --convert-btn-color: #3369ad7d;
  --hover-btn-color: #1d5aa58b;
  --hover-convert-btn-color: #2665b5d1;
  --image-size: cover;
  /** --image-url: url("https://images.pexels.com/photos/2817421/pexels-photo-2817421.jpeg?auto=compress&cs=tinysrgb&w=1260&h=750& dpr=1"); */
  --image-url: url("https://images.unsplash.com/photo-1506744038136-46273834b3fb");
  --main-bg-color: #2222228e;
`);

const preset3 = createPreset(`
  --mui-palette-primary-main:#9644f1;
  --mui-palette-error-main: #9c27b0;
  --mui-palette-background-default: #2225;
  --mui-palette-secondary-main: #4a148c5c;
  --mui-filled-input-background: #3c003fa8;
  --mui-focus-visible: #673ab7c2;
  --mui-palette-primary-dark: #330066;

  --autofill-color: #eb37ff1c;
  --convert-btn-color: #ab2b7e6e;
  --hover-btn-color: #8b51fb8b;
  --hover-convert-btn-color: #7d00c9a3;
  --image-size: cover;
  --image-url: url("https://images.pexels.com/photos/6162265/pexels-photo-6162265.jpeg?auto=compress&cs=tinysrgb");
  --main-bg-color: #2223;
`);

const preset4 = createPreset(`
  --mui-palette-primary-main: rgb(185, 185, 90);
  --mui-palette-error-main: #ff5722;
  --mui-palette-background-default: #2020208a;
  --mui-palette-secondary-main: #64dd178a;
  --mui-filled-input-background: #31313161;
  --mui-focus-visible: #6d6d6d94;
  --mui-palette-primary-dark: #3dcb5e;

  --autofill-color: #a19c0038;
  --convert-btn-color: #94ce7c6e;
  --hover-btn-color:rgba(161, 196, 66, 0.31);
  --hover-convert-btn-color: #81c462a3;
  --image-position-x: center;
  --image-position-y: center;
  --image-size: cover;
  --image-url: url('https://images.pexels.com/photos/973324/pexels-photo-973324.jpeg?auto=compress&cs=tinysrgb&w=1260&h=750&dpr=1');
  --main-bg-color: #222a;
`);

const PRESETS = {
  '0': STORAGE.get(PUB_CACHE_OBJ.customCss) ?? '',
  '1': preset1,
  '2': preset2,
  '3': preset3,
  '4': preset4,
} as const;

const normalize = (select?: string | null) => {
  switch (select) {
    case '1':
    case '2':
    case '3':
    case '4':
      return select;
    default:
      return '0';
  }
};

export const CSS = {
  normalize,

  preset: {
    /** Get current preset */
    get: () => normalize(STORAGE.get(PUB_CACHE_OBJ.presetNumber)),
    /** Set current preset */
    set: (presetN: keyof typeof PRESETS) => STORAGE.set(PUB_CACHE_OBJ.presetNumber, presetN),
  },

  css: {
    /** ID of the HTML from which the CSS is drawn. */
    id: 'user-custom-css',

    /** Get css */
    get: (presetN: keyof typeof PRESETS) => PRESETS[presetN],
    /** Set css */
    set: (css: string) => STORAGE.set(PUB_CACHE_OBJ.customCss, css),
  },
} as const;

export type CssPresets = keyof typeof PRESETS;
