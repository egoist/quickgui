import { defineConfig } from 'vite'
import { reactRouter } from '@react-router/dev/vite'
import tailwindcss from '@tailwindcss/vite'
import { cloudflare } from '@cloudflare/vite-plugin'
import mdx from '@mdx-js/rollup'
import rehypeShiki from '@shikijs/rehype'
import rehypeSlug from 'rehype-slug'
import remarkGfm from 'remark-gfm'
import { componentGuidePlugin } from './scripts/component-guide-plugin.ts'

export default defineConfig({
  resolve: { tsconfigPaths: true },
  plugins: [
    cloudflare({
      configPath: './wrangler.dev.jsonc',
      viteEnvironment: { name: 'ssr' },
    }),
    tailwindcss(),
    mdx({
      remarkPlugins: [remarkGfm, componentGuidePlugin],
      rehypePlugins: [
        rehypeSlug,
        [
          rehypeShiki,
          {
            themes: {
              light: 'github-light-high-contrast',
              dark: 'github-dark-high-contrast',
            },
            defaultColor: false,
          },
        ],
      ],
    }),
    reactRouter(),
  ],
})
