import { BrowserWindow, ipcMain } from 'electron';
import type { Status } from './types/patch_listener';

function emitStatus(window: BrowserWindow, eventName: string, status: Status) {
  window.webContents.send(eventName, status);
}

// Example: emit status from backend operations
ipcMain.handle('patch:listener', async (_, args) => {
  const win = BrowserWindow.getFocusedWindow();
  if (!win) {
    return;
  }

  emitStatus(win, 'd_merge://progress/patch', { type: 'ReadingPatches', content: { index: 1, total: 10 } });
});
