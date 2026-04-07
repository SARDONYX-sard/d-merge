import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite-plus';

export default defineConfig({
  plugins: [react()],
  test: {
    alias: [{ find: '@/', replacement: `${__dirname}/src/` }],
    globals: true,
    root: `./src/`,
    environment: 'jsdom',
    setupFiles: [`${__dirname}/vitest.setup.mts`],
    reporters: ['default', 'hanging-process'],
  },
});
