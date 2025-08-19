import { contextBridge, ipcRenderer } from 'electron';
import type { ModIds, ModInfo, PatchArguments, PatchOptions } from './cmd/types/patch';
import type { Status } from './cmd/types/patch_listener';
import type { OutputFormat, TreeViewBaseItem } from './cmd/types/serde_hkx';

// frontend <-> backend bridge functions
// frontend: window.__ELECTRON__.showContextMenu()
contextBridge.exposeInMainWorld('__ELECTRON__', {
  // dialog
  async save(options: Tauri.SaveDialogOptions = {}): Promise<string | null> {
    return await ipcRenderer.invoke('dialog:save', options);
  },
  async open(options: Tauri.OpenDialogOptions = {}): Promise<string | string[] | null> {
    return await ipcRenderer.invoke('dialog:open', options);
  },

  // fs
  async readFile(path: string): Promise<string> {
    return await ipcRenderer.invoke('fs:readFile', path);
  },
  async writeFile(path: string, content: string): Promise<void> {
    return await ipcRenderer.invoke('fs:writeFile', { path, content });
  },

  // log
  async changeLogLevel(level?: string) {
    return ipcRenderer.invoke('log:changeLevel', level);
  },
  // log dependencies
  async getAppLogDir(): Promise<string> {
    return await ipcRenderer.invoke('app:getLogDir');
  },
  async getAppName(): Promise<string> {
    return await ipcRenderer.invoke('app:getName');
  },

  // --- Patch / Skyrim APIs ---
  async getSkyrimDir(runtime: 'SkyrimSE' | 'SkyrimLE'): Promise<string> {
    return ipcRenderer.invoke('skyrim:getDataDir', runtime);
  },
  async loadModsInfo(searchGlob: string): Promise<ModInfo[]> {
    return ipcRenderer.invoke('patch:loadModsInfo', searchGlob);
  },
  async patch(outputDir: string, ids: ModIds, options: PatchOptions): Promise<void> {
    return ipcRenderer.invoke('patch:patch', { outputDir, ids, options } as const satisfies PatchArguments);
  },
  async cancelPatch(): Promise<void> {
    return ipcRenderer.invoke('patch:cancel');
  },
  async setVfsMode(value: boolean): Promise<void> {
    return ipcRenderer.invoke('patch:setVfsMode', value);
  },

  // --- Patch Status Listener ---
  async statusListener(eventName: string, f: (status: Status) => void): Promise<() => void> {
    const listener = (_event: Electron.IpcRendererEvent, status: Status) => f(status);
    event: ipcRenderer.on(eventName, listener);
    return () => {
      ipcRenderer.removeListener(eventName, listener);
    };
  },

  // --- serde_hkx APIs ---
  async convert(inputs: string[], output: string, format: OutputFormat, roots?: string[]): Promise<void> {
    return ipcRenderer.invoke('serde_hkx:convert', { inputs, output, format, roots });
  },

  async loadDirNode(dirs: string[]): Promise<TreeViewBaseItem[]> {
    return ipcRenderer.invoke('serde_hkx:loadDirNode', { dirs });
  },

  // shell
  async openPath(path: string): Promise<void> {
    return await ipcRenderer.invoke('opener:openPath', path);
  },
  async openUrl(path: string): Promise<void> {
    return await ipcRenderer.invoke('opener:openUrl', path);
  },

  showContextMenu: async () => await ipcRenderer.invoke('show-context-menu'),
  zoom: async (delta: number) => await ipcRenderer.invoke('zoom', { delta }),
});
