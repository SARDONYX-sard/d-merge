import path from 'node:path';
import { app, BrowserWindow, BrowserWindowConstructorOptions, ipcMain } from 'electron';
import { isVfsMode } from './cmd/patch';

let win: BrowserWindow | null = null;

const WINDOW_CONFIG = {
  title: 'D Merge',
  backgroundColor: 'transparent',
  width: 825,
  height: 920,
  show: false,
  webPreferences: {
    preload: path.join(__dirname, 'preload.js'),
  },
} as const satisfies BrowserWindowConstructorOptions;

export const createWindow = () => {
  win = new BrowserWindow(WINDOW_CONFIG);

  if (app.isPackaged) {
    win.loadURL(`file://${path.resolve(__dirname, '../frontend/index.html')}`);
  } else {
    win.loadURL(`http://localhost:3000/`);
    // win.webContents.openDevTools();
  }

  win.show();

  // Close request with button ✕
  win.on('close', (event) => {
    // In order to support automatic setting loading in the MO2 virtual environment,
    // first export the settings in the frontend and then close the window.
    if (isVfsMode) {
      event.preventDefault(); // not close by default
      // Call the same event name as tauri.
      // This is the event currently used in frontend's `useBackup.ts`.
      win?.webContents.send('tauri://close-requested');
    }
  });

  return win;
};

// Really close when a destroy request is received from the Renderer
ipcMain.handle('window-destroy', () => {
  win?.destroy();
  win = null;
});
