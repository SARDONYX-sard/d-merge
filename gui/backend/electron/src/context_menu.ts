import { BrowserWindow, Menu, MenuItem, MenuItemConstructorOptions } from 'electron';

export type ContextMenuParams = { x: number; y: number; selectionText: string };

/** Webview like context menu */
export const DEFAULT_MENU = Menu.buildFromTemplate([
  {
    label: 'Back',
    accelerator: 'Alt+Left',
    click: (_item, focusedWindow) => {
      if (focusedWindow instanceof BrowserWindow && focusedWindow.webContents.canGoBack()) {
        focusedWindow.webContents.goBack();
      }
    },
  },
  {
    label: 'Reload',
    accelerator: 'Ctrl+R',
    click: (_item, focusedWindow) => {
      focusedWindow instanceof BrowserWindow && focusedWindow.webContents.reload();
    },
  },
  { label: 'Minimize', role: 'minimize' },
  { label: 'Zoom', role: 'zoom' },
  // Unsupported these operations, ['Save as', 'Print', 'Share]

  { type: 'separator' },
  {
    label: 'Inspect with Developer Tools',
    accelerator: 'F12',
    click: (_item, focusedWindow) => {
      focusedWindow instanceof BrowserWindow && focusedWindow.webContents.openDevTools({ mode: 'detach' });
    },
  },
] as const satisfies Array<MenuItemConstructorOptions | MenuItem>);

/** Selecting text menu */
export const TEXT_MENU = Menu.buildFromTemplate([
  { label: 'Cut', role: 'cut', accelerator: 'Ctrl+X' },
  { label: 'Copy', role: 'copy', accelerator: 'Ctrl+C' },
  { label: 'Paste', role: 'paste', accelerator: 'Ctrl+V' },
  { label: 'Paste as Plain Text', role: 'pasteAndMatchStyle', accelerator: 'Ctrl+Shift+V' },
  { type: 'separator' },
  { label: 'Select All', role: 'selectAll', accelerator: 'Ctrl+A' },
  { type: 'separator' },
  {
    label: 'Inspect with Developer Tools',
    accelerator: 'F12',
    click: (_item, focusedWindow) =>
      focusedWindow instanceof BrowserWindow && focusedWindow.webContents.openDevTools({ mode: 'detach' }),
  },
] as const satisfies Array<MenuItemConstructorOptions | MenuItem>);
