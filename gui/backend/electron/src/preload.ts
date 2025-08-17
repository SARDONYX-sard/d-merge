import { contextBridge, ipcRenderer } from 'electron';

// frontend <-> backend functions
contextBridge.exposeInMainWorld('__ELECTRON__', {
  foo: 'bar',
  showContextMenu: async () => await ipcRenderer.invoke('show-context-menu'),
  zoom: async (delta: number) => await ipcRenderer.invoke('zoom'),
});
