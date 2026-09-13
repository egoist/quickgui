import { describe, expect, test } from 'bun:test'
import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import GithubSlugger from 'github-slugger'
import { SUPPORTED_LOCALES } from '../src/i18n'
import {
  ALL_COMPONENT_DOCS,
  SWIFT_UI_COMPONENTS,
  UI_COMPONENTS,
  componentDocsPath,
} from '../src/lib/component-docs'
import { DOCS_FRONTENDS, docsPages, docsPath, switchDocsFrontend } from '../src/lib/docs'
import { docsNavGroups, docsPageArea } from '../src/lib/docs-navigation'
import { DOCS_GUIDE_ORDER } from '../src/lib/docs-structure'
import { localizedDocsPage } from '../src/lib/docs-locales'
import { resolveDocsRoute } from '../src/lib/docs-routing'
import { searchDocs } from '../src/lib/docs-search'

const root = resolve(import.meta.dir, '..')

describe('frontend documentation routes', () => {
  test('defaults to Go and canonicalizes getting-started URLs', () => {
    expect(resolveDocsRoute()).toEqual({ kind: 'redirect', path: '/docs/go' })
    for (const frontend of DOCS_FRONTENDS) {
      expect(resolveDocsRoute(frontend)?.kind).toBe('page')
      expect(resolveDocsRoute(frontend, 'getting-started')).toEqual({
        kind: 'redirect',
        path: `/docs/${frontend}`,
      })
    }
    expect(resolveDocsRoute('unknown')).toBeUndefined()
    expect(resolveDocsRoute('typescript', 'swift-ui')?.kind).toBe('page')
  })

  test('redirects every previous Go guide and component URL', () => {
    for (const page of docsPages('go')) {
      expect(resolveDocsRoute(page.slug)).toEqual({
        kind: 'redirect',
        path: docsPath('go', page.slug),
      })
    }
    for (const component of ALL_COMPONENT_DOCS) {
      const family = component.kind === 'ui' ? 'components' : 'swift-ui'
      expect(resolveDocsRoute(family, component.slug)).toEqual({
        kind: 'redirect',
        path: componentDocsPath(component),
      })
    }
    expect(resolveDocsRoute('components', 'missing')).toBeUndefined()
    expect(resolveDocsRoute('unknown', 'styling')).toBeUndefined()
  })

  test('switches shared topics and falls back for unknown pages', () => {
    expect(switchDocsFrontend('/docs/go/styling', 'typescript')).toBe('/docs/typescript/styling')
    expect(switchDocsFrontend('/docs/typescript/ui', 'go')).toBe('/docs/go/rendering')
    expect(switchDocsFrontend('/docs/go', 'typescript')).toBe('/docs/typescript')
    expect(switchDocsFrontend('/docs/typescript', 'go')).toBe('/docs/go')
    expect(switchDocsFrontend('/docs/go/components/button', 'typescript')).toBe(
      '/docs/typescript/components/button',
    )
    expect(switchDocsFrontend('/docs/go/swift-ui', 'typescript')).toBe('/docs/typescript/swift-ui')
    expect(switchDocsFrontend('/docs/typescript/native-services', 'go')).toBe(
      '/docs/go/native-services',
    )
    expect(switchDocsFrontend('/docs/typescript/missing', 'go')).toBe('/docs/go')
  })

  test('redirects the former UI guide to Rendering in either frontend', () => {
    for (const frontend of DOCS_FRONTENDS) {
      expect(resolveDocsRoute(frontend, 'ui')).toEqual({
        kind: 'redirect',
        path: `/docs/${frontend}/rendering`,
      })
      for (const slug of ['reactivity', 'rendering', 'components', 'routing', 'styling']) {
        expect(resolveDocsRoute(frontend, slug)?.kind).toBe('page')
        expect(switchDocsFrontend(`/docs/go/${slug}`, frontend)).toBe(`/docs/${frontend}/${slug}`)
      }
    }
    expect(resolveDocsRoute('ui')).toEqual({ kind: 'redirect', path: '/docs/go/rendering' })
  })
})

test('guide pages map to the Guide, Components, or SwiftUI docs area', () => {
  expect(docsPageArea('getting-started')).toBe('guide')
  expect(docsPageArea('forms-and-input')).toBe('guide')
  expect(docsPageArea('overlays-and-dialogs')).toBe('guide')
  expect(docsPageArea('extensions')).toBe('guide')
  expect(docsPageArea('components')).toBe('components')
  expect(docsPageArea('swift-ui')).toBe('swift-ui')
  expect(docsPageArea('swift-ui-hosting')).toBe('swift-ui')
})

test('Go, TypeScript, and Rust share guide order, localized sections, and sidebar structure', () => {
  const pages = Object.fromEntries(DOCS_FRONTENDS.map((frontend) => [frontend, docsPages(frontend)]))
  for (const frontend of DOCS_FRONTENDS) {
    expect(pages[frontend].map((page) => page.slug)).toEqual(DOCS_GUIDE_ORDER)
  }
  for (const locale of SUPPORTED_LOCALES) {
    for (let index = 0; index < pages.go.length; index++) {
      for (const frontend of DOCS_FRONTENDS) {
        expect(localizedDocsPage(pages.go[index], locale).title).toBe(
          localizedDocsPage(pages[frontend][index], locale).title,
        )
        expect(localizedDocsPage(pages.go[index], locale).outline).toEqual(
          localizedDocsPage(pages[frontend][index], locale).outline,
        )
        expect(switchDocsFrontend(docsPath('go', pages.go[index].slug), frontend)).toBe(
          docsPath(frontend, pages[frontend][index].slug),
        )
      }
    }
    const structure = (
      frontend: (typeof DOCS_FRONTENDS)[number],
      area: 'guide' | 'components' | 'swift-ui',
    ) =>
      docsNavGroups(locale, frontend, area).map((group) => ({
        id: group.id,
        title: group.title,
        titles: group.items.map((item) => item.title),
        paths: group.items.map((item) => item.path.replace(`/docs/${frontend}`, '')),
      }))
    for (const frontend of DOCS_FRONTENDS) {
      for (const area of ['guide', 'components', 'swift-ui'] as const) {
        expect(structure('go', area)).toEqual(structure(frontend, area))
      }
    }
  }
})

test('guide, components, and SwiftUI each have a dedicated sidebar', async () => {
  for (const frontend of DOCS_FRONTENDS) {
    for (const locale of SUPPORTED_LOCALES) {
      const guides = docsNavGroups(locale, frontend, 'guide')
      const components = docsNavGroups(locale, frontend, 'components')
      const swiftUi = docsNavGroups(locale, frontend, 'swift-ui')
      const guidePaths = guides.flatMap((group) => group.items.map((item) => item.path))
      const componentPaths = components.flatMap((group) => group.items.map((item) => item.path))
      const swiftUiPaths = swiftUi.flatMap((group) => group.items.map((item) => item.path))

      expect(guides.find((group) => group.id === 'guides')!.items.map((item) => item.path)).toEqual(
        [
          'reactivity',
          'rendering',
          'styling',
          'forms-and-input',
          'overlays-and-dialogs',
          'routing',
          'animations',
          'native-services',
        ].map((slug) => `/docs/${frontend}/${slug}`),
      )
      expect(guides.find((group) => group.id === 'advanced')!.items.map((item) => item.path)).toEqual(
        ['app-icon', 'bundled-resources', 'updater', 'extensions'].map(
          (slug) => `/docs/${frontend}/${slug}`,
        ),
      )
      expect(guidePaths).not.toContain(`/docs/${frontend}/components`)
      expect(guidePaths).not.toContain(`/docs/${frontend}/swift-ui`)
      expect(guidePaths).not.toContain(`/docs/${frontend}/swift-ui-hosting`)
      expect(guidePaths.every((path) => !path.includes('/components/') && !path.includes('/swift-ui/'))).toBe(
        true,
      )

      expect(components.find((group) => group.id === 'overview')!.items.map((item) => item.path)).toEqual([
        `/docs/${frontend}/components`,
      ])
      const componentReference = components.filter((group) => group.id === 'components')
      expect(componentReference).toHaveLength(1)
      expect(componentReference[0].items.map((item) => item.path).sort()).toEqual(
        UI_COMPONENTS.map((component) => componentDocsPath(component, frontend)).sort(),
      )
      const names = componentReference[0].items.map((item) => item.title)
      expect(names).toEqual([...names].sort((a, b) => a.localeCompare(b, 'en')))
      expect(componentPaths.every((path) => path.includes('/components'))).toBe(true)
      expect(componentPaths.some((path) => path.includes('/swift-ui'))).toBe(false)

      expect(swiftUi.find((group) => group.id === 'swift-ui')!.items.map((item) => item.path)).toEqual([
        `/docs/${frontend}/swift-ui`,
        `/docs/${frontend}/swift-ui-hosting`,
      ])
      expect(swiftUi.find((group) => group.id === 'swift-ui-components')!.items.map((item) => item.path)).toEqual(
        SWIFT_UI_COMPONENTS.map((component) => componentDocsPath(component, frontend)),
      )
      expect(swiftUiPaths.every((path) => path.includes('/swift-ui'))).toBe(true)
      expect(swiftUiPaths.some((path) => path.includes('/components'))).toBe(false)

      for (const paths of [guidePaths, componentPaths, swiftUiPaths]) {
        expect(new Set(paths).size).toBe(paths.length)
      }
      if (locale === 'en') {
        expect(guides.map((group) => group.title)).toEqual(['Introduction', 'Guides', 'Advanced'])
        expect(components.map((group) => group.title)).toEqual(['Overview', 'Components'])
        expect(swiftUi.map((group) => group.title)).toEqual(['SwiftUI', 'SwiftUI Components'])
      }
      for (const component of UI_COMPONENTS) {
        const content = await readFile(
          resolve(
            root,
            'src/content/docs',
            frontend,
            'components',
            locale === 'en' ? '' : locale,
            'ui',
            `${component.slug}.mdx`,
          ),
          'utf8',
        )
        expect(content.trim().length).toBeGreaterThan(0)
      }
    }
  }
})

test('localized guides keep matching outlines and examples', async () => {
  for (const frontend of DOCS_FRONTENDS) {
    for (const source of docsPages(frontend)) {
      let englishExamples: string[] = []
      for (const locale of SUPPORTED_LOCALES) {
        const content = await readFile(
          resolve(root, 'src/content/docs', frontend, locale, `${source.slug}.mdx`),
          'utf8',
        )
        const page = localizedDocsPage(source, locale)
        const slugger = new GithubSlugger()
        const headings = [...content.matchAll(/^## (.+)$/gm)].map((match) => ({
          id: slugger.slug(match[1]),
          title: match[1],
        }))
        expect(headings).toEqual(page.outline)
        if (
          !['reactivity', 'rendering', 'components', 'routing', 'styling'].includes(source.slug) &&
          !(frontend === 'typescript' && source.slug === 'updater')
        )
          continue
        const examples = [...content.matchAll(/```[^\n]*\n[\s\S]*?```/g)].map((match) => match[0])
        if (locale === 'en') englishExamples = examples
        else expect(examples).toEqual(englishExamples)
      }
    }
  }
})

test('all internal MDX links use valid frontend routes', async () => {
  const paths = new Set([
    ...DOCS_FRONTENDS.flatMap((frontend) =>
      docsPages(frontend).map((page) => docsPath(frontend, page.slug)),
    ),
    ...DOCS_FRONTENDS.flatMap((frontend) =>
      ALL_COMPONENT_DOCS.map((component) => componentDocsPath(component, frontend)),
    ),
  ])
  const contentRoot = resolve(root, 'src/content/docs')
  for await (const file of new Bun.Glob('**/*.mdx').scan(contentRoot)) {
    const content = await readFile(resolve(contentRoot, file), 'utf8')
    const links = [...content.matchAll(/\]\((\/docs[^\s)]*)\)/g)]
    for (const [, link] of links) {
      const path = link.split(/[?#]/, 1)[0]
      expect(paths.has(path), `${file}: ${link}`).toBe(true)
    }
  }
})

test('every Go component has a localized TypeScript and Rust reference and keeps its route when switching', async () => {
  for (const component of ALL_COMPONENT_DOCS) {
    for (const frontend of ['typescript', 'rust'] as const) {
      const path = componentDocsPath(component, frontend)
      expect(switchDocsFrontend(componentDocsPath(component), frontend)).toBe(path)
      expect(switchDocsFrontend(path, 'go')).toBe(componentDocsPath(component))
      const fence = frontend === 'typescript' ? 'tsx' : 'rust'
      let example = ''
      for (const locale of SUPPORTED_LOCALES) {
        const file = resolve(
          root,
          'src/content/docs',
          frontend,
          'components',
          locale === 'en' ? '' : locale,
          component.kind,
          `${component.slug}.mdx`,
        )
        const content = await readFile(file, 'utf8')
        expect(content).not.toContain('```go')
        const code = [...content.matchAll(new RegExp('```' + fence + '\\n([\\s\\S]*?)```', 'g'))]
          .map((match) => match[1])
          .join('\n')
        expect(code.trim().length).toBeGreaterThan(0)
        if (locale === 'en') example = code
        else expect(code).toBe(example)
      }
    }
  }
})

test('search scopes results and caches by frontend and locale', async () => {
  const originalFetch = globalThis.fetch
  const requested: string[] = []
  globalThis.fetch = (async (input: string | URL | Request) => {
    const path = String(input)
    requested.push(path)
    return new Response(Bun.file(resolve(root, 'public', path.slice(1))))
  }) as typeof fetch
  try {
    for (const frontend of DOCS_FRONTENDS) {
      for (const locale of SUPPORTED_LOCALES) {
        const prefix = `${locale === 'en' ? '' : `/${locale}`}/docs/${frontend}`
        const hits = await searchDocs(locale, frontend, 'transition')
        expect(hits.length).toBeGreaterThan(0)
        expect(hits.every((hit) => hit.document.url.startsWith(prefix))).toBe(true)
      }
    }
    const typescript = await searchDocs('en', 'typescript', 'button')
    expect(typescript.every((hit) => !hit.document.url.includes('/docs/go/'))).toBe(true)
    const checkbox = await searchDocs('en', 'typescript', 'checkbox')
    expect(
      checkbox.some((hit) => hit.document.url.startsWith('/docs/typescript/components/checkbox')),
    ).toBe(true)
    expect(requested.sort()).toEqual(
      DOCS_FRONTENDS.flatMap((frontend) =>
        SUPPORTED_LOCALES.map((locale) => `/docs-search/${frontend}/${locale}.json`),
      ).sort(),
    )
  } finally {
    globalThis.fetch = originalFetch
  }
})
