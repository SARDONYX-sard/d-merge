import react from '@vitejs/plugin-react-swc';
import tsconfigPaths from 'vite-tsconfig-paths';
import { defineConfig } from 'vitest/config';

const srcPath = `${__dirname}/gui/frontend/src/`;

export default defineConfig({
  plugins: [react(), tsconfigPaths()],
  test: {
    alias: [{ find: '@/', replacement: srcPath }],
    globals: true,
    root: `./src/`,
    environment: 'jsdom',
    setupFiles: ['./vitest.setup.mts', 'tests/vitest.customMatchers.ts'],
    testTransformMode: { ssr: ['**/*'] },
    reporters: ['default', 'hanging-process'],
  },
});
