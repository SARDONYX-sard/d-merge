import { convert, DirEntry, loadDirNode } from 'd_merge_node';
import { BrowserWindow, ipcMain } from 'electron';
import type { OutputFormat, Payload } from './types/serde_hkx';

type ConvertOptions = { inputs: string[]; output: string; format: OutputFormat; roots?: string[] };

ipcMain.handle('serde_hkx:convert', async (_, { inputs, output, format, roots }: ConvertOptions) => {
  const win = BrowserWindow.getFocusedWindow();
  if (!win) {
    throw new Error('The window you are focusing on cannot be found. Please keep the GUI app clicked.');
  }

  await convert(inputs, output, format, roots, ({ pathId, status }) => {
    // status: string → enum number
    // We need to do the same thing as tauri's serde processing.
    // tauri uses serde_repr for enum.
    // In other words, you hash the path and use it as the ID to notify the frontend of the progress (0-3).
    win.webContents.send('d_merge://progress/convert', {
      pathId,
      status: status.valueOf(),
    } as const satisfies Payload);
  });
});

// Load directory tree
ipcMain.handle('serde_hkx:loadDirNode', async (_, { dirs }: { dirs: string[] }): Promise<DirEntry[]> => {
  return loadDirNode(dirs);
});
