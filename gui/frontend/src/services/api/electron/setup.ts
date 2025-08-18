export const isElectron = () => {
  //@ts-ignore
  return (globalThis || window).__ELECTRON__ !== undefined;
};

// @ts-ignore
export const electronApi = window.__ELECTRON__;

if (isElectron()) {
  window.addEventListener('contextmenu', (e) => {
    e.preventDefault();
    electronApi.showContextMenu();
  });

  window.addEventListener(
    'wheel',
    (e) => {
      if (e.ctrlKey) {
        if (e.deltaY < 0) {
          const delta = e.deltaY < 0 ? 0.1 : -0.1;
          electronApi.zoom(delta);
        }
      }
    },
    { passive: false },
  );
}
