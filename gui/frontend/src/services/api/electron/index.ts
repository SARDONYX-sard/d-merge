export const isElectron = () => {
  //@ts-ignore
  return (globalThis || window).__ELECTRON__ !== undefined;
};

// @ts-ignore
export const electronApi = window.__ELECTRON__;
