import { frontendLabel } from "../lib/docs";
import { redirect } from 'react-router'
import { resolveDocsRoute } from '../lib/docs-routing'
import type { Route } from './+types/docs'
import docsCss from '../docs.css?url'
import { DocsShell } from '../components/docs/docs-shell'
import { getDocsMdxComponents } from '../components/docs/mdx-components'
import {
  OG_LOCALES,
  SUPPORTED_LOCALES,
  resolveLocale,
  type Locale,
} from '../i18n'
import { findDocsPage, docsPath } from '../lib/docs'
import { docsPageArea } from '../lib/docs-navigation'
import { localizedDocsPage } from '../lib/docs-locales'
import { guideMdx } from '../lib/docs-mdx'
import { siteMeta } from '../lib/meta'
import { readFrontendPreference } from '../lib/frontend-preference'

function localizedPath(locale: Locale, path: string): string {
  return locale === 'en' ? path : `/${locale}${path}`
}

export function loader({ params, request }: Route.LoaderArgs) {
  const locale = resolveLocale(params.locale)
  const route = resolveDocsRoute(
    params.frontend,
    params.slug,
    readFrontendPreference(request.headers.get('Cookie')),
  )
  if (!locale || !route) throw new Response('Page not found', { status: 404 })
  if (route.kind === 'redirect') {
    if (!params.frontend && !params.slug) {
      return redirect(localizedPath(locale, route.path) + new URL(request.url).search, {
        status: 302,
        headers: { 'Cache-Control': 'private, no-store' },
      })
    }
    return redirect(localizedPath(locale, route.path) + new URL(request.url).search, 308)
  }
  const page = route.page

  return {
    locale,
    origin: new URL(request.url).origin,
    slug: page.slug,
    frontend: page.frontend,
  }
}

export const links: Route.LinksFunction = () => [
  { rel: 'stylesheet', href: docsCss },
]

export const meta: Route.MetaFunction = ({ loaderData }) => {
  if (!loaderData) return []
  const source = findDocsPage(loaderData.frontend, loaderData.slug)
  if (!source) return []

  const { locale, origin } = loaderData
  const page = localizedDocsPage(source, locale)
  const path = docsPath(page.frontend, page.slug)
  const title = `${page.title} | QuickGUI ${frontendLabel(page.frontend)}`

  return [
    ...siteMeta(origin, title),
    { name: 'description', content: page.description },
    { property: 'og:title', content: title },
    { property: 'og:description', content: page.description },
    { property: 'og:locale', content: OG_LOCALES[locale] },
    {
      tagName: 'link',
      rel: 'canonical',
      href: `${origin}${localizedPath(locale, path)}`,
    },
    ...SUPPORTED_LOCALES.map((other) => ({
      tagName: 'link' as const,
      rel: 'alternate',
      hrefLang: other,
      href: `${origin}${localizedPath(other, path)}`,
    })),
    {
      tagName: 'link',
      rel: 'alternate',
      hrefLang: 'x-default',
      href: `${origin}${path}`,
    },
  ]
}

export default function DocsRoute({ loaderData }: Route.ComponentProps) {
  const source = findDocsPage(loaderData.frontend, loaderData.slug)
  if (!source) return null
  const page = localizedDocsPage(source, loaderData.locale)

  const Content = guideMdx(source.frontend, source.slug, loaderData.locale)
  return (
    <DocsShell
      frontend={loaderData.frontend}
      locale={loaderData.locale}
      page={{
        title: page.title,
        description: page.description,
        outline: page.outline,
        path: docsPath(page.frontend, page.slug),
        area: docsPageArea(page.slug),
      }}
    >
      <Content components={getDocsMdxComponents(loaderData.locale)} />
    </DocsShell>
  )
}
