import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import path from 'path'

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { '@': path.resolve(__dirname, './src') },
  },
  base: '/admin/',
  build: {
    outDir: '../crates/api/static/admin',
    emptyOutDir: true,
  },
  server: {
    proxy: {
      '/admin/api': 'http://localhost:8081',
      '/oauth2': 'http://localhost:8081',
      '/.well-known': 'http://localhost:8081',
    },
  },
})
