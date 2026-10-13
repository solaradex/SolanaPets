import { defineConfig } from 'vite';

// SolanaPets game client — local dev only. No deployment targets configured.
export default defineConfig({
  base: './',
  server: {
    host: true,
    port: 5173,
  },
  build: {
    target: 'es2020',
    outDir: 'dist',
  },
});
