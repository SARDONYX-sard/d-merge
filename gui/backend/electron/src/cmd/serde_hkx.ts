import { convert, loadDirNode } from 'd_merge_node';
import { ipcMain } from 'electron';

ipcMain.handle('serde_hkx:convert', async (_, { inputs, output, format, roots }) => {
  await convert(inputs, output, format, roots);
});

// Load directory tree
ipcMain.handle('serde_hkx:loadDirNode', async (_, { dirs }: { dirs: string[] }) => {
  return await loadDirNode(dirs);
});
