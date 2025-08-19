import { rm } from 'node:fs/promises';
import path from 'node:path';
import { behaviorGen, Config, cancelPatch, getSkyrimDataDir, loadModsInfo } from 'd_merge_node';
import { app, BrowserWindow, ipcMain } from 'electron';
import { ModInfo, PatchArguments } from './types/patch';
import { Status } from './types/patch_listener';

/**
 * Removes a file or directory at the given path if it exists.
 * This function uses `fs.promises.rm` with `recursive` and `force` to avoid errors
 * if the target does not exist.
 *
 * @param path - The file or directory path to remove.
 */
async function removeIfExists(path: string) {
  try {
    await rm(path, { recursive: true, force: true });
    console.log(`Removed: ${path}`);
  } catch (err) {
    console.error(`Failed to remove ${path}:`, err);
  }
}

/**
 * Removes the `meshes` folder and `.d_merge/.debug` folder inside the output directory
 * if `autoRemoveMeshes` is set to true.
 * This mimics the Rust behavior of automatically cleaning up generated assets.
 *
 * @param outputDir - The output directory where `meshes` and `.d_merge/.debug` are located.
 * @param autoRemoveMeshes - Boolean flag indicating whether to remove these folders.
 */
async function handleAutoRemoveMeshes(outputDir: string, autoRemoveMeshes: boolean) {
  if (autoRemoveMeshes) {
    const meshesPath = path.join(outputDir, 'meshes');
    const debugPath = path.join(outputDir, '.d_merge', '.debug');
    await Promise.all([removeIfExists(meshesPath), removeIfExists(debugPath)]);
  }
}

/**
 * Returns the path to the application's assets directory.
 * - In packaged mode, it uses the `resourcesPath` provided by Electron.
 * - In development mode, it resolves the relative path from the current source directory.
 *
 * @returns The absolute path to the assets directory.
 */
function getAssetDir(): string {
  if (app.isPackaged) {
    return path.join(process.resourcesPath, 'assets');
  } else {
    return path.join(__dirname, '../../../../../resource/assets');
  }
}

// Skyrim path
ipcMain.handle('skyrim:getDataDir', async (_, runtime: 'SkyrimSE' | 'SkyrimLE') => {
  return await getSkyrimDataDir(runtime);
});

// Mods info
ipcMain.handle('patch:loadModsInfo', async (_, glob: string): Promise<ModInfo[]> => {
  return await loadModsInfo(glob);
});

// Patch operation
ipcMain.handle('patch:patch', async (_, { outputDir, ids, options }: PatchArguments) => {
  const { hackOptions, debug, outputTarget, autoRemoveMeshes, useProgressReporter } = options;

  if (autoRemoveMeshes) {
    await handleAutoRemoveMeshes(outputDir, true);
  }

  // Progress
  const win = BrowserWindow.getFocusedWindow();
  if (!win) {
    throw new Error('The window you are focusing on cannot be found. Please keep the GUI app clicked.');
  }
  const statusReporter = useProgressReporter
    ? (status: Status) => {
        win.webContents.send('d_merge://progress/patch', status);
      }
    : undefined;

  const config = {
    resourceDir: getAssetDir(),
    outputDir,
    statusReport: statusReporter,
    debug,
    hackOptions,
    outputTarget,
  } as const satisfies Config;

  return await behaviorGen(ids, config);
});

ipcMain.handle('patch:cancel', async () => {
  await cancelPatch();
});

ipcMain.handle('patch:setVfsMode', async (_, { enabled }) => {
  // setVfsMode(enabled);
});
