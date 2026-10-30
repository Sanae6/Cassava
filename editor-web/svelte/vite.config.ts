import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'
import { viteSingleFile } from "vite-plugin-singlefile"
import tailwindcss from '@tailwindcss/vite'
import svelteFluent from '@nubolab-ffwd/svelte-fluent/vite';
import path from 'path'

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    svelte(),
    tailwindcss(),
    svelteFluent(),
    viteSingleFile()
  ],
  resolve: {
    alias: [
      { find: '$bindings', replacement: path.resolve(__dirname, 'bindings') },
    ],
  },
})
