export const isElectron = () => {
  //@ts-ignore
  return (globalThis || window).__ELECTRON__ !== undefined;
};

// @ts-ignore
export const electronApi = window.__ELECTRON__;

if (isElectron()) {
  window.addEventListener('contextmenu', (e) => {
    e.preventDefault();

    const selectionText = window.getSelection()?.toString() || '';

    electronApi.showContextMenu({
      x: e.x,
      y: e.y,
      selectionText,
    });
  });

  window.addEventListener(
    'wheel',
    (e) => {
      if (e.ctrlKey) {
        // scroll up → zoom in, scroll down → zoom out
        const delta = e.deltaY < 0 ? 0.05 : -0.05;
        electronApi.zoom(delta);
      }
    },
    { passive: false },
  );
}
