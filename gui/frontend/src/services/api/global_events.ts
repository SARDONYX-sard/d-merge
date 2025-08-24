import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { electronApi, isElectron } from '@/services/api/electron';

if (isElectron()) {
  window.addEventListener('contextmenu', (e) => {
    e.preventDefault();

    const { x, y } = e;
    const selectionText = window.getSelection()?.toString() || '';
    electronApi.showContextMenu({ x, y, selectionText });
  });
}

window.addEventListener(
  'wheel',
  async (e) => {
    if (e.ctrlKey) {
      // -   scroll up → zoom in
      // - scroll down → zoom out
      const delta = e.deltaY < 0 ? 0.05 : -0.05;

      if (isTauri()) {
        await getCurrentWebview().setZoom(delta);
      } else if (isElectron()) {
        await electronApi.zoom(delta);
      }
    }
  },
  { passive: false },
);
