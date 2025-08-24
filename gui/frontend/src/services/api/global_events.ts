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

// -   scroll up → zoom in
// - scroll down → zoom out
let currentZoom = 1.0; // initial magnification
window.addEventListener(
  'wheel',
  async (e) => {
    if (e.ctrlKey) {
      if (isTauri()) {
        currentZoom *= e.deltaY < 0 ? 1.05 : 0.95;
        currentZoom = Math.min(Math.max(currentZoom, 0.1), 5); // limitation
        console.log('currentZoom', currentZoom);
        await getCurrentWebview().setZoom(currentZoom);
        return;
      }

      if (isElectron()) {
        await electronApi.zoom(e.deltaY < 0 ? 0.05 : -0.05);
        return;
      }
    }
  },
  { passive: false },
);
