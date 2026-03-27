import { defineConfig } from "vite";
import { tanstackStart } from '@tanstack/react-start/plugin/vite'
import tailwindcss from "@tailwindcss/vite";
import viteTsConfigPaths from 'vite-tsconfig-paths'
import viteReact from '@vitejs/plugin-react'

export default defineConfig({
  // SSR configuration to handle CSS imports
  ssr: {
    noExternal: [],
    external: [
      'solid-js',
      'solid-js/web',
      '@tanstack/router-devtools',
      '@tanstack/router-devtools-core',
      '@tanstack/react-router-devtools',
      '@copilotkit/react-ui',
      '@copilotkit/react-core',
      '@copilotkit/runtime-client-gql',
      'katex',
      'rehype-katex',
    ],
  },

  server: {
    port: 3000,
  },

  plugins: [
    viteTsConfigPaths({
      projects: ['./tsconfig.json'],
    }),
    tailwindcss(),
    tanstackStart(),
    viteReact(),
  ],

  resolve: {
    alias: {
      // Handle KaTeX CSS imports to prevent Node.js module resolution errors
      "katex/dist/katex.min.css": "katex/dist/katex.css",
      "katex/dist/katex.css": "katex/dist/katex.css",
    }
  },

  build: {
    rollupOptions: {
      external: [
        "node:stream",
        "node:stream/web",
        "node:path",
        "node:fs",
        "node:async_hooks",
        "tanstack-start-injected-head-scripts:v",
        "solid-js",
        "solid-js/web",
        /^@copilotkit\/.*/,
      ],
    },
    commonjsOptions: {
      transformMixedEsModules: true,
      include: [/katex/, /node_modules/],
    },
  },

  optimizeDeps: {
    exclude: ['katex', 'solid-js', '@tanstack/router-devtools'],
  },
});
