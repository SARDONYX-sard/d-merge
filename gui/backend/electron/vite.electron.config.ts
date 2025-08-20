import { resolve } from 'node:path';
import { defineConfig } from 'vite';

export default defineConfig({
  build: {
    outDir: 'out', // electron-builder
    emptyOutDir: true,
    lib: {
      entry: {
        main: resolve(__dirname, 'src/main.ts'),
        preload: resolve(__dirname, 'src/preload.ts'),
      },
      formats: ['cjs'], // Electron main is CommonJS
    },
    rollupOptions: {
      external: [
        'electron', // no need `electron` inclusion
        /^node:/,
        // 'd_merge_node', // native module
      ],
      output: {
        entryFileNames: '[name].js',
      },
    },
    minify: true,
  },
});
