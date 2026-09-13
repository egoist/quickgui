import { docsOutline, docsTitle, DOCS_GUIDE_ORDER } from './docs-structure'
import type { Locale } from '../i18n'
import { RUST_DOCS_PAGES } from './rust-docs'
import { TYPESCRIPT_DOCS_PAGES } from './typescript-docs'

export const DOCS_FRONTENDS = ['go', 'typescript', 'rust'] as const
export type DocsFrontend = (typeof DOCS_FRONTENDS)[number]

export type DocsSlug = (typeof DOCS_GUIDE_ORDER)[number]

export interface DocsOutlineItem {
  id: string
  title: string
  level?: 2 | 3
}

export interface DocsPageTranslation {
  title: string
  description: string
  outline: readonly DocsOutlineItem[]
  searchTerms: readonly string[]
}

export interface DocsPageMeta extends DocsPageTranslation {
  frontend: DocsFrontend
  slug: DocsSlug
  translations?: Partial<
    Record<Locale, Omit<DocsPageTranslation, 'searchTerms' | 'outline' | 'title'>>
  >
}

const goDescriptions = {
  'getting-started': 'Create a native desktop app with Go and open your first window.',
  'project-structure': 'The files in a QuickGUI Go project and how to configure them.',
  components: 'Write components, compose children, and use the built-in control set.',
  reactivity: 'Keep UI in sync with signals, memos, effects, and batched updates.',
  rendering: 'Show and hide content, render lists, and work with the current window.',
  styling: 'Lay out and style views with Flexbox, Grid, and interaction states.',
  'forms-and-input': 'Build text fields, checkboxes, radios, and select controls.',
  'overlays-and-dialogs': 'Show popovers, dialogs, and operating-system file pickers.',
  routing: 'Declare routes, nested layouts, and in-app navigation.',
  animations: 'Animate hover, color, and opacity, and play GIF or WebP images.',
  'native-services': 'Open windows and use menus, clipboard, dialogs, and file watching.',
  'swift-ui': 'Embed real SwiftUI controls inside a Go application.',
  'swift-ui-hosting': 'Style SwiftUI controls and nest QuickGUI content inside them.',
  'app-icon': 'Use resources/icon.png as the packaged application icon, or change the window or Dock icon at runtime.',
  'bundled-resources': 'Ship files in resources/ and load them from the packaged resource directory.',
  updater: 'Ship signed automatic updates for macOS, Windows, and Linux.',
  extensions: 'Share Go components or add a native service your app can call.',
} satisfies Record<DocsSlug, string>

const goSearchTerms = {
  'getting-started': ['install', 'create', 'cli', 'window', 'bun', 'macos'],
  'project-structure': ['files', 'config', 'entry', 'package', 'quickgui.config'],
  components: ['view', 'text', 'button', 'tabs', 'checkbox'],
  reactivity: ['signal', 'memo', 'effect', 'batch', 'cleanup'],
  rendering: ['show', 'for', 'list', 'window', 'children'],
  styling: ['style', 'layout', 'flexbox', 'grid', 'color', 'hover'],
  'forms-and-input': ['input', 'field', 'checkbox', 'radio', 'select', 'events'],
  'overlays-and-dialogs': ['popover', 'dialog', 'overlay', 'alert', 'file picker'],
  routing: ['router', 'route', 'layout', 'outlet', 'navigation', 'history', 'parameters'],
  animations: ['transition', 'animation', 'hover', 'opacity', 'gif', 'webp'],
  'native-services': ['window', 'menu', 'clipboard', 'WatchFiles', 'Dispatch', 'Async'],
  'swift-ui': ['swiftui', 'host', 'slider', 'toggle', 'picker'],
  'swift-ui-hosting': ['modifier', 'glass', 'quickguihostview', 'popover'],
  'app-icon': ['icon', 'icns', 'ico', 'dock', 'taskbar', 'png'],
  'bundled-resources': ['resources', 'assets', 'bundle', 'fonts', 'resourceDir'],
  updater: ['updater', 'sparkle', 'appcast', 'updates', 'signing'],
  extensions: ['extension', 'init-extension', 'plugin', 'manifest', 'InvokeExtension'],
} satisfies Record<DocsSlug, readonly string[]>

export const GO_DOCS_PAGES: readonly DocsPageMeta[] = DOCS_GUIDE_ORDER.map((slug) => ({
  frontend: 'go',
  slug,
  outline: docsOutline(slug),
  title: docsTitle(slug),
  description: goDescriptions[slug],
  searchTerms: goSearchTerms[slug],
}))

export function isDocsFrontend(value: string | undefined): value is DocsFrontend {
  return (DOCS_FRONTENDS as readonly string[]).includes(value ?? '')
}

export function docsPages(frontend: DocsFrontend): readonly DocsPageMeta[] {
  if (frontend === 'typescript') return TYPESCRIPT_DOCS_PAGES
  if (frontend === 'rust') return RUST_DOCS_PAGES
  return GO_DOCS_PAGES
}

export function frontendLabel(frontend: DocsFrontend): string {
  if (frontend === 'typescript') return 'TypeScript'
  if (frontend === 'rust') return 'Rust'
  return 'Go'
}

export function docsPath(frontend: DocsFrontend, slug: DocsSlug = 'getting-started'): string {
  const root = `/docs/${frontend}`
  return slug === 'getting-started' ? root : `${root}/${slug}`
}

export function findDocsPage(frontend: DocsFrontend, slug?: string): DocsPageMeta | undefined {
  const normalized = slug === 'ui' ? 'rendering' : slug || 'getting-started'
  return docsPages(frontend).find((page) => page.slug === normalized)
}

// Preserve the current guide or component when switching language frontends.
export function switchDocsFrontend(path: string, frontend: DocsFrontend): string {
  const [, , , slug, component] = path.split('/')
  if (component && (slug === 'components' || slug === 'swift-ui')) {
    return `/docs/${frontend}/${slug}/${component}`
  }
  const page = findDocsPage(frontend, slug)
  return docsPath(frontend, page?.slug)
}
