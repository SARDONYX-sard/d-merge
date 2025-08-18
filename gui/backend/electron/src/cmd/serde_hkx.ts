import { ipcMain } from 'electron';

ipcMain.handle('convert:convert', async (_, { inputs, output, format, roots }) => {
  // TODO: Rust FFI
  // await convert('convert', { inputs, output, format, roots });
});

// Load directory tree
ipcMain.handle('convert:loadDirNode', async (_, { dirs }: { dirs: string[] }) => {
  // TODO: Rust FFI
  // return await invoke<TreeViewBaseItem[]>(dirs); // TODO: Return TreeViewBaseItem[]
});
