import { dialog, ipcMain } from 'electron';

ipcMain.handle('dialog:save', async (_, tauriOpts: Tauri.SaveDialogOptions) => {
  const electronOpts = mapSaveOptions(tauriOpts);
  const result = await dialog.showSaveDialog(electronOpts);
  return result.canceled ? null : result.filePath;
});

/** Tauri.SaveDialogOptions -> Electron options */
const mapSaveOptions = (tauriOpts: Tauri.SaveDialogOptions): Electron.SaveDialogOptions => {
  const { title, filters, defaultPath, canCreateDirectories } = tauriOpts;

  const properties: Electron.SaveDialogOptions['properties'] = [];
  if (canCreateDirectories) {
    properties.push('createDirectory'); // Support macOS only
  }

  return {
    title,
    filters,
    defaultPath,
    properties,
  };
};

ipcMain.handle('dialog:open', async (_, tauriOpts: Tauri.OpenDialogOptions): Promise<string | string[] | null> => {
  const electronOpts = mapOpenOptions(tauriOpts);
  const result = await dialog.showOpenDialog(electronOpts);
  return result.canceled ? null : result.filePaths.length > 1 ? result.filePaths : result.filePaths[0];
});

/** Tauri.OpenDialogOptions -> Electron options */
const mapOpenOptions = (tauriOpts: Tauri.OpenDialogOptions): Electron.OpenDialogOptions => {
  const { canCreateDirectories, defaultPath, directory, filters, multiple, recursive: _, title } = tauriOpts;

  const properties: Electron.OpenDialogOptions['properties'] = [];
  if (canCreateDirectories) {
    properties.push('createDirectory'); // Support macOS only
  }
  if (multiple) {
    properties.push('multiSelections');
  }
  if (directory) {
    properties.push('openDirectory');
  }

  return {
    title,
    filters,
    defaultPath,
    properties,
  };
};
