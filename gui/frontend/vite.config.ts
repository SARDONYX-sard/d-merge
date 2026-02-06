import { tanstackRouter } from '@tanstack/router-plugin/vite';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';
import tsconfigPaths from 'vite-tsconfig-paths';

// var ref: https://v2.tauri.app/reference/environment-variables/#tauri-cli-hook-commands
const IS_DEBUG = !!process.env.TAURI_ENV_DEBUG;

export default defineConfig({
  plugins: [
    tanstackRouter({
      target: 'react',
      autoCodeSplitting: true,
    }),
    react(),
    tsconfigPaths(),
  ],
  build: {
    sourcemap: IS_DEBUG,
  },
});
