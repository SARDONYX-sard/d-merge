import { changeLogLevel } from 'd_merge_node';
import { app, ipcMain } from 'electron';

export const LOG_DIR = app.getPath('logs');
export const LOG_FILE_NAME = `d_merge.log`;

ipcMain.handle('log:changeLevel', async (_, level: string = 'error') => {
  let isValid = false;
  if (['error', 'warn', 'info', 'debug', 'trace'].includes(level)) {
    isValid = true;
  }
  changeLogLevel(level);
});

ipcMain.handle('app:getLogDir', () => LOG_DIR);
ipcMain.handle('app:getName', () => 'd_merge');
