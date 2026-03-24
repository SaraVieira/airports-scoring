import path from 'path'
import { defineConfig } from 'vite'
import { tanstackStart } from '@tanstack/react-start/plugin/vite'
import { nitro } from 'nitro/vite'
import viteReact from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  envDir: '..',
  resolve: {
    alias: {
      '~': path.resolve(__dirname, 'app'),
    },
  },
  server: {
    port: 3000,
  },
  plugins: [
    tailwindcss(),
    tanstackStart({
      srcDirectory: 'app',
    }),
    nitro({ preset: 'node-server' }),
    viteReact(),
  ],
})
