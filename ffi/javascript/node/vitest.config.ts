// This file is a test configuration file for gui/frontend.
// By placing the configuration file in the root directory, it eliminates wasted time in directory searches
// and prevents time delays in testing.

import tsconfigPaths from 'vite-tsconfig-paths';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [tsconfigPaths()],
  test: {
    globals: true,
    root: './',
    environment: 'node',
    testTransformMode: { ssr: ['**/*'] },
    reporters: ['default', 'hanging-process'],
    testTimeout: 30 * 1000, // 30 seconds
  },
});
