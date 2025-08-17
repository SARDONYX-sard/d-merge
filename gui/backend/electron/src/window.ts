import path from 'node:path';
import { app, BrowserWindow, BrowserWindowConstructorOptions } from 'electron';
import tauriConfigJson from '../../tauri/tauri.conf.json';

let win: BrowserWindow | null = null;

const WINDOW_CONFIG = {
  title: 'D Merge',
  backgroundColor: 'transparent',
  width: tauriConfigJson.app.windows[0].width,
  height: tauriConfigJson.app.windows[0].height,
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
  return win;
};
