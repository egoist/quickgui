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

export const GO_DOCS_PAGES: readonly DocsPageMeta[] = [
  {
    frontend: 'go',
    slug: 'getting-started',
    outline: docsOutline('getting-started'),
    title: docsTitle('getting-started'),
    description: 'Create and run a native QuickGUI app with QuickGUI UI.',
    searchTerms: ['install', 'create', 'cli', 'window', 'bun', 'macos'],
  },
  {
    frontend: 'go',
    slug: 'project-structure',
    outline: docsOutline('project-structure'),
    title: docsTitle('project-structure'),
    description: 'Understand the files in a generated QuickGUI UI project.',
    searchTerms: ['files', 'config', 'entry', 'package', 'quickgui.config'],
  },
  {
    frontend: 'go',
    slug: 'updater',
    outline: docsOutline('updater'),
    title: docsTitle('updater'),
    description: 'Add optional Sparkle-compatible updates to a Go application.',
    searchTerms: ['updater', 'sparkle', 'appcast', 'ed25519', 'extension', 'updates'],
  },
  {
    frontend: 'go',
    slug: 'extensions',
    outline: docsOutline('extensions'),
    title: docsTitle('extensions'),
    description: 'Share Go components and build optional native providers for QuickGUI.',
    searchTerms: [
      'extension',
      'authoring',
      'init-extension',
      'zig',
      'rust',
      'plugin',
      'native provider',
      'third-party',
      'purego',
      'manifest',
      'ABI',
      'ServiceApi',
      'RequireExtension',
      'InvokeExtension',
      'OpenExtension',
      'npm',
    ],
  },
  {
    frontend: 'go',
    slug: 'native-services',
    title: docsTitle('native-services'),
    description: 'Manage window lifetimes and asynchronous native work.',
    outline: docsOutline('native-services'),
    searchTerms: ['window', 'native', 'lifecycle', 'services', 'WatchFiles', 'Dispatch', 'Async'],
  },
  {
    frontend: 'go',
    slug: 'reactivity',
    outline: docsOutline('reactivity'),
    title: docsTitle('reactivity'),
    description:
      'Signals connect state to the text and properties that read it. Updates change retained nodes without rerunning the entire component.',
    searchTerms: ['signal', 'reactivity', 'memo', 'effect', 'batch', 'cleanup'],
  },
  {
    frontend: 'go',
    slug: 'rendering',
    outline: docsOutline('rendering'),
    title: docsTitle('rendering'),
    description:
      'Components construct a retained tree once when mounted. Reactive bindings update the affected nodes; the native core handles layout, painting, and accessibility.',
    searchTerms: ['rendering', 'retained', 'children', 'mount', 'lifecycle', 'keyed'],
  },
  {
    frontend: 'go',
    slug: 'components',
    outline: docsOutline('components'),
    title: docsTitle('components'),
    description: 'Choose between primitives and accessible compound components.',
    searchTerms: ['view', 'text', 'button', 'tabs', 'checkbox', 'unstyled'],
  },
  {
    frontend: 'go',
    slug: 'routing',
    outline: docsOutline('routing'),
    title: docsTitle('routing'),
    description:
      'The router selects components from the current application path and keeps a navigation history. Routes render native QuickGUI content in the current window.',
    searchTerms: ['router', 'route', 'layout', 'outlet', 'navigation', 'history', 'parameters'],
  },
  {
    frontend: 'go',
    slug: 'styling',
    outline: docsOutline('styling'),
    title: docsTitle('styling'),
    description: 'Lay out and style native nodes with familiar properties.',
    searchTerms: ['style', 'layout', 'flexbox', 'grid', 'color', 'hover'],
  },
  {
    frontend: 'go',
    slug: 'animations',
    outline: docsOutline('animations'),
    title: docsTitle('animations'),
    description: 'Animate native style changes and images with retained Go components.',
    searchTerms: [
      'transition',
      'animation',
      'easing',
      'duration',
      'hover',
      'opacity',
      'reduced motion',
      'gif',
      'webp',
    ],
  },
  {
    frontend: 'go',
    slug: 'forms-and-input',
    outline: docsOutline('forms-and-input'),
    title: docsTitle('forms-and-input'),
    description: 'Build controlled fields, choices, and selection controls.',
    searchTerms: ['input', 'field', 'checkbox', 'radio', 'select', 'events'],
  },
  {
    frontend: 'go',
    slug: 'overlays-and-dialogs',
    outline: docsOutline('overlays-and-dialogs'),
    title: docsTitle('overlays-and-dialogs'),
    description: 'Present in-window overlays and operating-system dialogs.',
    searchTerms: ['popover', 'dialog', 'overlay', 'alert', 'file picker'],
  },
  {
    frontend: 'go',
    slug: 'swift-ui',
    outline: docsOutline('swift-ui'),
    title: docsTitle('swift-ui'),
    description: 'Mount real SwiftUI controls inside a QuickGUI UI application.',
    searchTerms: ['swiftui', 'host', 'slider', 'toggle', 'picker', 'native'],
  },
  {
    frontend: 'go',
    slug: 'swift-ui-hosting',
    outline: docsOutline('swift-ui-hosting'),
    title: docsTitle('swift-ui-hosting'),
    description: 'Style SwiftUI controls and host QuickGUI content back inside them.',
    searchTerms: ['modifier', 'glass', 'quickguihostview', 'popover', 'reverse host'],
  },
]

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
