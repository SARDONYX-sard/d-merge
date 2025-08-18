import { app, ipcMain } from 'electron';

ipcMain.handle('log:changeLevel', (_, level: string = 'error') => {
  let isValid = false;
  if (['error', 'warn', 'info', 'debug', 'trace'].includes(level)) {
    isValid = true;
  }

  // TODO: implement log level change logic in Electron(By Rust tracing ffi)
});

ipcMain.handle('app:getLogDir', () => app.getPath('logs'));
ipcMain.handle('app:getName', () => app.getName());
