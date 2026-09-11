import { useEffect, useId, useLayoutEffect, useRef, useState, type ReactNode } from 'react'
import { Link, useNavigate } from 'react-router'
import { Logo } from '../logo'
import { docsNavGroups } from '../../lib/docs-navigation'
import {
  DOCS_FRONTENDS,
  docsPath,
  frontendLabel,
  isDocsFrontend,
  switchDocsFrontend,
  type DocsFrontend,
  type DocsOutlineItem,
} from '../../lib/docs'
import { localePath, type Locale } from '../../i18n'
import { site } from '../../lib/site'
import { LanguageMenu } from '../language-menu'
import { prefetchDocsSearch, searchDocs, type DocsSearchHit } from '../../lib/docs-search'
import { rememberFrontend } from '../../lib/frontend-preference'

export type DocsArea = 'guide' | 'components' | 'swift-ui'

export interface DocsShellPage {
  title: string
  description: string
  outline: readonly DocsOutlineItem[]
  path: string
  area: DocsArea
}

const sidebarScrollTop = new Map<string, number>()
const collapsedSidebarGroups = new Map<DocsFrontend, Set<string>>()

const ui = {
  en: {
    guide: 'Guide',
    components: 'Components',
    swiftUi: 'SwiftUI',
    search: 'Search',
    searchDocs: 'Search documentation',
    noResults: 'No documentation found.',
    searchPrompt: 'Search every guide, component, and API.',
    searching: 'Searching documentation…',
    searchError: 'Search is unavailable. Try again.',
    searchResults: '{{count}} results',
    closeSearch: 'Close search',
    onThisPage: 'On this page',
    menu: 'Menu',
    previous: 'Previous page',
    next: 'Next page',
    skip: 'Skip to content',
    language: 'Language',
    frontend: 'App language',
    home: 'QuickGUI home',
    documentation: 'Documentation',
    documentationPages: 'Documentation pages',
    github: 'QuickGUI on GitHub',
  },
  zh: {
    guide: '指南',
    components: '组件',
    swiftUi: 'SwiftUI',
    search: '搜索',
    searchDocs: '搜索文档',
    noResults: '未找到相关文档。',
    searchPrompt: '搜索所有指南、组件和 API。',
    searching: '正在搜索文档…',
    searchError: '搜索暂时不可用，请重试。',
    searchResults: '{{count}} 个结果',
    closeSearch: '关闭搜索',
    onThisPage: '本页内容',
    menu: '菜单',
    previous: '上一页',
    next: '下一页',
    skip: '跳到正文',
    language: '语言',
    frontend: '应用语言',
    home: 'QuickGUI 首页',
    documentation: '文档',
    documentationPages: '文档页面',
    github: 'QuickGUI 的 GitHub 仓库',
  },
  ja: {
    guide: 'ガイド',
    components: 'コンポーネント',
    swiftUi: 'SwiftUI',
    search: '検索',
    searchDocs: 'ドキュメントを検索',
    noResults: '該当するドキュメントはありません。',
    searchPrompt: 'すべてのガイド、コンポーネント、API を検索します。',
    searching: 'ドキュメントを検索中…',
    searchError: '検索を利用できません。もう一度お試しください。',
    searchResults: '{{count}} 件の結果',
    closeSearch: '検索を閉じる',
    onThisPage: 'このページの内容',
    menu: 'メニュー',
    previous: '前のページ',
    next: '次のページ',
    skip: '本文へスキップ',
    language: '言語',
    frontend: 'アプリの言語',
    home: 'QuickGUI ホーム',
    documentation: 'ドキュメント',
    documentationPages: 'ドキュメントページ',
    github: 'QuickGUI の GitHub リポジトリ',
  },
} as const

function localize(locale: Locale, path: string): string {
  return locale === 'en' ? path : `/${locale}${path}`
}

function DocsSidebar({
  currentPath,
  locale,
  frontend,
  mobile = false,
  onNavigate,
}: {
  currentPath: string
  locale: Locale
  frontend: DocsFrontend
  mobile?: boolean
  onNavigate?: () => void
}) {
  const sidebarRef = useRef<HTMLElement>(null)
  const navigate = useNavigate()
  const pickerId = useId()
  const scrollKey = `${frontend}:${mobile ? 'mobile' : 'desktop'}`

  useLayoutEffect(() => {
    const collapsed = collapsedSidebarGroups.get(frontend)
    const groups = sidebarRef.current?.querySelectorAll<HTMLDetailsElement>('.docs-nav-group')
    for (const group of groups ?? []) {
      group.open =
        Boolean(group.querySelector('[aria-current="page"]')) || !collapsed?.has(group.dataset.group!)
    }
  }, [currentPath, frontend, mobile])

  useLayoutEffect(() => {
    if (sidebarRef.current) {
      sidebarRef.current.scrollTop = sidebarScrollTop.get(scrollKey) ?? 0
    }
  }, [scrollKey])

  function rememberScrollPosition() {
    if (sidebarRef.current) {
      sidebarScrollTop.set(scrollKey, sidebarRef.current.scrollTop)
    }
  }

  return (
    <aside
      ref={sidebarRef}
      className={mobile ? 'docs-mobile-drawer' : 'docs-sidebar'}
      aria-label={mobile ? ui[locale].menu : undefined}
      onScroll={rememberScrollPosition}
    >
      <div className="docs-frontend-picker">
        <label htmlFor={pickerId}>{ui[locale].frontend}</label>
        <div className="docs-frontend-select">
          <select
            id={pickerId}
            value={frontend}
            onChange={(event) => {
              const next = event.target.value
              if (!isDocsFrontend(next)) return
              rememberFrontend(next)
              rememberScrollPosition()
              onNavigate?.()
              void navigate(localize(locale, switchDocsFrontend(currentPath, next)))
            }}
          >
            {DOCS_FRONTENDS.map((value) => (
              <option key={value} value={value}>{frontendLabel(value)}</option>
            ))}
          </select>
          <span className="i-lucide-chevrons-up-down" aria-hidden />
        </div>
      </div>
      <nav aria-label={ui[locale].menu}>
        {docsNavGroups(locale, frontend).map((group) => (
          <details
            className="docs-nav-group"
            data-group={group.id}
            key={group.id}
            open
            onToggle={(event) => {
              const collapsed = collapsedSidebarGroups.get(frontend) ?? new Set<string>()
              if (event.currentTarget.open) collapsed.delete(group.id)
              else collapsed.add(group.id)
              collapsedSidebarGroups.set(frontend, collapsed)
              rememberScrollPosition()
            }}
          >
            <summary>
              <h2>{group.title}</h2>
              <span className="i-lucide-chevron-right" aria-hidden />
            </summary>
            <ul>
              {group.items.map((item) => (
                <li key={item.path}>
                  <Link
                    to={localize(locale, item.path)}
                    aria-current={currentPath === item.path ? 'page' : undefined}
                    onClick={() => {
                      rememberScrollPosition()
                      onNavigate?.()
                    }}
                  >
                    {item.title}
                  </Link>
                </li>
              ))}
            </ul>
          </details>
        ))}
      </nav>
    </aside>
  )
}

function DocsOutline({
  page,
  locale,
  mobile = false,
  onNavigate,
}: {
  page: DocsShellPage
  locale: Locale
  mobile?: boolean
  onNavigate?: () => void
}) {
  const content = (
    <nav aria-label={ui[locale].onThisPage}>
      <h2>
        <span className="i-lucide-list-filter" aria-hidden />
        {ui[locale].onThisPage}
      </h2>
      <ul>
        {page.outline.map((item) => (
          <li key={item.id} data-level={item.level ?? 2}>
            <a href={`#${item.id}`} onClick={onNavigate}>
              {item.title}
            </a>
          </li>
        ))}
      </ul>
    </nav>
  )

  return mobile ? (
    <aside className="docs-mobile-drawer docs-mobile-outline">{content}</aside>
  ) : (
    <aside className="docs-outline">{content}</aside>
  )
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

function HighlightedText({ text, query }: { text: string; query: string }) {
  const terms = query
    .trim()
    .split(/\s+/)
    .filter(Boolean)
    .sort((left, right) => right.length - left.length)

  if (!terms.length) return text
  const pattern = new RegExp(`(${terms.map(escapeRegExp).join('|')})`, 'gi')

  return text
    .split(pattern)
    .map((part, index) =>
      terms.some((term) => term.toLocaleLowerCase() === part.toLocaleLowerCase()) ? (
        <mark key={`${part}-${index}`}>{part}</mark>
      ) : (
        part
      ),
    )
}

function resultSnippet(hit: DocsSearchHit, query: string): string {
  const source = hit.document.content || hit.document.keywords
  if (!source) return ''

  const lowerSource = source.toLocaleLowerCase()
  const terms = query
    .trim()
    .toLocaleLowerCase()
    .split(/\s+/)
    .filter((term) => term.length > 1)
  const match = terms.reduce((best, term) => {
    const index = lowerSource.indexOf(term)
    return index >= 0 && index < best ? index : best
  }, Number.POSITIVE_INFINITY)
  const center = Number.isFinite(match) ? match : 0
  const start = Math.max(0, center - 45)
  const end = Math.min(source.length, start + 150)
  const snippet = source.slice(start, end).trim()
  return `${start ? '…' : ''}${snippet}${end < source.length ? '…' : ''}`
}

function SearchDialog({
  open,
  locale,
  frontend,
  onClose,
}: {
  open: boolean
  locale: Locale
  frontend: DocsFrontend
  onClose: () => void
}) {
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<DocsSearchHit[]>([])
  const [status, setStatus] = useState<'idle' | 'loading' | 'ready' | 'error'>('idle')
  const [activeIndex, setActiveIndex] = useState(0)
  const inputRef = useRef<HTMLInputElement>(null)
  const dialogRef = useRef<HTMLElement>(null)
  const resultsId = useId()
  const navigate = useNavigate()
  const labels = ui[locale]

  useEffect(() => {
    if (!open) return
    const previouslyFocused = document.activeElement as HTMLElement | null
    const previousOverflow = document.body.style.overflow
    setQuery('')
    setResults([])
    setStatus('idle')
    setActiveIndex(0)
    document.body.style.overflow = 'hidden'
    prefetchDocsSearch(locale, frontend)
    const frame = requestAnimationFrame(() => inputRef.current?.focus())

    return () => {
      cancelAnimationFrame(frame)
      document.body.style.overflow = previousOverflow
      previouslyFocused?.focus()
    }
  }, [locale, frontend, open])

  useEffect(() => {
    if (!open) return
    const term = query.trim()
    if (!term) {
      setResults([])
      setStatus('idle')
      setActiveIndex(0)
      return
    }

    let cancelled = false
    setStatus('loading')
    const timeout = window.setTimeout(() => {
      void searchDocs(locale, frontend, term)
        .then((hits) => {
          if (cancelled) return
          setResults(hits)
          setStatus('ready')
          setActiveIndex(0)
        })
        .catch(() => {
          if (!cancelled) setStatus('error')
        })
    }, 70)

    return () => {
      cancelled = true
      window.clearTimeout(timeout)
    }
  }, [locale, frontend, open, query])

  useEffect(() => {
    if (!open || !results[activeIndex]) return
    document.getElementById(`${resultsId}-${activeIndex}`)?.scrollIntoView({ block: 'nearest' })
  }, [activeIndex, open, results, resultsId])

  if (!open) return null

  function selectResult(hit: DocsSearchHit) {
    onClose()
    void navigate(hit.document.url)
  }

  function resultArea(hit: DocsSearchHit): string {
    if (hit.document.area === 'swift-ui') return labels.swiftUi
    if (hit.document.area === 'ui') return labels.components
    return labels.guide
  }

  return (
    <div className="docs-search-backdrop" onMouseDown={onClose}>
      <section
        ref={dialogRef}
        className="docs-search-dialog"
        role="dialog"
        aria-modal="true"
        aria-label={labels.searchDocs}
        onMouseDown={(event) => event.stopPropagation()}
        onKeyDown={(event) => {
          if (event.key === 'Escape') {
            event.preventDefault()
            event.stopPropagation()
            onClose()
            return
          }
          if (event.key !== 'Tab') return

          const focusable = dialogRef.current?.querySelectorAll<HTMLElement>(
            'input, button, a[href]:not([tabindex="-1"])',
          )
          if (!focusable?.length) return
          const first = focusable[0]
          const last = focusable[focusable.length - 1]
          if (event.shiftKey && document.activeElement === first) {
            event.preventDefault()
            last.focus()
          } else if (!event.shiftKey && document.activeElement === last) {
            event.preventDefault()
            first.focus()
          }
        }}
      >
        <div className="docs-search-field">
          <span className="i-lucide-search" aria-hidden />
          <input
            ref={inputRef}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'ArrowDown' && results.length) {
                event.preventDefault()
                setActiveIndex((current) => (current + 1) % results.length)
              } else if (event.key === 'ArrowUp' && results.length) {
                event.preventDefault()
                setActiveIndex((current) => (current === 0 ? results.length - 1 : current - 1))
              } else if (event.key === 'Home' && results.length) {
                event.preventDefault()
                setActiveIndex(0)
              } else if (event.key === 'End' && results.length) {
                event.preventDefault()
                setActiveIndex(results.length - 1)
              } else if (event.key === 'Enter' && results[activeIndex]) {
                event.preventDefault()
                selectResult(results[activeIndex])
              }
            }}
            placeholder={labels.searchDocs}
            aria-label={labels.searchDocs}
            role="combobox"
            aria-autocomplete="list"
            aria-controls={resultsId}
            aria-expanded={results.length > 0}
            aria-activedescendant={results[activeIndex] ? `${resultsId}-${activeIndex}` : undefined}
          />
          <button type="button" onClick={onClose} aria-label={labels.closeSearch}>
            Esc
          </button>
        </div>
        <div className="docs-search-results">
          <div className="docs-search-status" aria-live="polite">
            {status === 'loading'
              ? labels.searching
              : status === 'ready' && results.length
                ? labels.searchResults.replace('{{count}}', String(results.length))
                : ''}
          </div>

          {status === 'idle' ? (
            <div className="docs-search-empty">
              <span className="i-lucide-files" aria-hidden />
              <p>{labels.searchPrompt}</p>
            </div>
          ) : null}
          {status === 'ready' && !results.length ? (
            <div className="docs-search-empty">
              <span className="i-lucide-search-x" aria-hidden />
              <p>{labels.noResults}</p>
            </div>
          ) : null}
          {status === 'error' ? (
            <div className="docs-search-empty docs-search-error">
              <span className="i-lucide-circle-alert" aria-hidden />
              <p>{labels.searchError}</p>
            </div>
          ) : null}

          {results.length ? (
            <ul
              id={resultsId}
              role="listbox"
              aria-label={labels.searchResults.replace('{{count}}', String(results.length))}
            >
              {results.map((result, index) => {
                const title = result.document.section || result.document.pageTitle
                const snippet = resultSnippet(result, query)
                return (
                  <li
                    id={`${resultsId}-${index}`}
                    key={result.id}
                    role="option"
                    aria-selected={index === activeIndex}
                  >
                    <Link
                      to={result.document.url}
                      tabIndex={-1}
                      onClick={onClose}
                      onMouseMove={() => setActiveIndex(index)}
                    >
                      <span className="docs-search-result-path">
                        {resultArea(result)}
                        <span aria-hidden>›</span>
                        {result.document.pageTitle}
                      </span>
                      <strong>
                        <HighlightedText text={title} query={query} />
                      </strong>
                      {snippet ? (
                        <small>
                          <HighlightedText text={snippet} query={query} />
                        </small>
                      ) : null}
                    </Link>
                  </li>
                )
              })}
            </ul>
          ) : null}
        </div>
      </section>
    </div>
  )
}

function DocsHeader({
  area,
  currentPath,
  locale,
  frontend,
  menuOpen,
  onMenuToggle,
  onSearch,
  onSearchPrepare,
}: {
  area: DocsArea
  currentPath: string
  locale: Locale
  frontend: DocsFrontend
  menuOpen: boolean
  onMenuToggle: () => void
  onSearch: () => void
  onSearchPrepare: () => void
}) {
  const labels = ui[locale]
  const headerLinks = [
    { label: labels.guide, href: docsPath(frontend), area: 'guide' },
    { label: labels.components, href: docsPath(frontend, 'components'), area: 'components' },
    { label: labels.swiftUi, href: docsPath(frontend, 'swift-ui'), area: 'swift-ui' },
  ] as const

  return (
    <header className="docs-header">
      <div className="docs-frame docs-header-inner">
        <a href={localePath(locale)} className="docs-brand" aria-label={labels.home}>
          <Logo />
          <span>{site.name}</span>
        </a>

        <nav className="docs-header-nav" aria-label={labels.documentation}>
          {headerLinks.map((link) => (
            <a
              key={link.area}
              href={localize(locale, link.href)}
              aria-current={link.area === area ? 'page' : undefined}
            >
              {link.label}
            </a>
          ))}
        </nav>

        <div className="docs-header-actions">
          <button
            type="button"
            className="docs-search-button"
            onClick={onSearch}
            onFocus={onSearchPrepare}
            onPointerEnter={onSearchPrepare}
            aria-label={labels.searchDocs}
          >
            <span className="i-lucide-search" aria-hidden />
            <span>{labels.search}</span>
            <kbd>⌘ K</kbd>
          </button>
          <LanguageMenu
            locale={locale}
            label={labels.language}
            hrefForLocale={(candidate) => localize(candidate, currentPath)}
          />
          <a
            className="docs-github-link"
            href={site.links.github}
            target="_blank"
            rel="noreferrer"
            aria-label={labels.github}
          >
            <span className="i-simple-icons-github" aria-hidden />
          </a>
          <button
            type="button"
            className="docs-mobile-menu-button"
            onClick={onMenuToggle}
            aria-expanded={menuOpen}
            aria-label={labels.menu}
          >
            <span className={menuOpen ? 'i-lucide-x' : 'i-lucide-menu'} aria-hidden />
          </button>
        </div>
      </div>
    </header>
  )
}

function DocsPager({
  page,
  locale,
  frontend,
}: {
  page: DocsShellPage
  locale: Locale
  frontend: DocsFrontend
}) {
  const pages = docsNavGroups(locale, frontend).flatMap((group) => group.items)
  const index = pages.findIndex((candidate) => candidate.path === page.path)
  const previous = index > 0 ? pages[index - 1] : undefined
  const next = index >= 0 && index < pages.length - 1 ? pages[index + 1] : undefined

  return (
    <nav className="docs-pager" aria-label={ui[locale].documentationPages}>
      {previous ? (
        <a href={localize(locale, previous.path)} className="docs-pager-previous">
          <small>{ui[locale].previous}</small>
          <span>{previous.title}</span>
        </a>
      ) : (
        <span />
      )}
      {next ? (
        <a href={localize(locale, next.path)} className="docs-pager-next">
          <small>{ui[locale].next}</small>
          <span>{next.title}</span>
        </a>
      ) : null}
    </nav>
  )
}

export function DocsShell({
  page,
  locale,
  frontend,
  children,
}: {
  page: DocsShellPage
  locale: Locale
  frontend: DocsFrontend
  children: ReactNode
}) {
  const [mobilePanel, setMobilePanel] = useState<'menu' | 'outline' | null>(null)
  const [searchOpen, setSearchOpen] = useState(false)

  useEffect(() => {
    rememberFrontend(frontend)
  }, [frontend])

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
        event.preventDefault()
        setSearchOpen(true)
      } else if (event.key === 'Escape') {
        setSearchOpen(false)
        setMobilePanel(null)
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [])

  return (
    <div className="docs-root">
      <a className="docs-skip-link" href="#docs-content">
        {ui[locale].skip}
      </a>
      <DocsHeader
        area={page.area}
        currentPath={page.path}
        locale={locale}
        frontend={frontend}
        menuOpen={mobilePanel === 'menu'}
        onMenuToggle={() => setMobilePanel((current) => (current === 'menu' ? null : 'menu'))}
        onSearch={() => setSearchOpen(true)}
        onSearchPrepare={() => prefetchDocsSearch(locale, frontend)}
      />

      <div className="docs-mobile-local-nav">
        <button
          type="button"
          aria-expanded={mobilePanel === 'menu'}
          onClick={() => setMobilePanel((current) => (current === 'menu' ? null : 'menu'))}
        >
          <span className="i-lucide-list-filter" aria-hidden />
          {ui[locale].menu}
        </button>
        <button
          type="button"
          aria-expanded={mobilePanel === 'outline'}
          onClick={() => setMobilePanel((current) => (current === 'outline' ? null : 'outline'))}
        >
          {ui[locale].onThisPage}
          <span
            className={mobilePanel === 'outline' ? 'i-lucide-chevron-up' : 'i-lucide-chevron-right'}
            aria-hidden
          />
        </button>
      </div>

      <div className="docs-frame docs-layout">
        <DocsSidebar currentPath={page.path} locale={locale} frontend={frontend} />
        <div className="docs-content-column">
          <main id="docs-content" className="docs-article">
            <h1>{page.title}</h1>
            {children}
            <DocsPager page={page} locale={locale} frontend={frontend} />
          </main>
        </div>
        <DocsOutline page={page} locale={locale} />
      </div>

      {mobilePanel ? (
        <div className="docs-mobile-panel-backdrop" onMouseDown={() => setMobilePanel(null)}>
          <div onMouseDown={(event) => event.stopPropagation()}>
            {mobilePanel === 'menu' ? (
              <DocsSidebar
                currentPath={page.path}
                locale={locale}
                frontend={frontend}
                mobile
                onNavigate={() => setMobilePanel(null)}
              />
            ) : (
              <DocsOutline
                page={page}
                locale={locale}
                mobile
                onNavigate={() => setMobilePanel(null)}
              />
            )}
          </div>
        </div>
      ) : null}

      <SearchDialog
        open={searchOpen}
        locale={locale}
        frontend={frontend}
        onClose={() => setSearchOpen(false)}
      />
    </div>
  )
}
