// Vitest workspace configuration
// - Splits tests into two environments:
//   1. Node environment (backend / utilities)
//   2. React + jsdom environment (frontend)

import { defineConfig } from 'vitest/config';

// ref: https://vitest.dev/guide/projects
export default defineConfig({
  test: {
    projects: ['ffi/javascript/node', 'gui/frontend'],
  },
});
