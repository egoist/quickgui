import type { Locale } from '../i18n'
import {
  COMPONENT_NAV_GROUPS,
  SWIFT_UI_NAV_GROUP,
  componentDocsPath,
  type ComponentDoc,
} from './component-docs'
import { docsPath, findDocsPage, type DocsFrontend, type DocsSlug } from './docs'
import { localizedComponentDescription, localizedDocsPage } from './docs-locales'

export type DocsArea = 'guide' | 'components' | 'swift-ui'

interface DocsNavItem {
  title: string
  description: string
  path: string
  terms: string
}

export interface DocsNavGroup {
  id: 'introduction' | 'overview' | 'guides' | 'components' | 'swift-ui' | 'swift-ui-components' | 'advanced'
  title: string
  items: readonly DocsNavItem[]
}

const labels = {
  en: {
    introduction: 'Introduction',
    overview: 'Overview',
    guides: 'Guides',
    components: 'Components',
    swiftUi: 'SwiftUI',
    swiftComponents: 'SwiftUI Components',
    advanced: 'Advanced',
  },
  zh: {
    introduction: '简介',
    overview: '概览',
    guides: '指南',
    components: '组件',
    swiftUi: 'SwiftUI',
    swiftComponents: 'SwiftUI 组件',
    advanced: '进阶',
  },
  ja: {
    introduction: 'はじめに',
    overview: '概要',
    guides: 'ガイド',
    components: 'コンポーネント',
    swiftUi: 'SwiftUI',
    swiftComponents: 'SwiftUI コンポーネント',
    advanced: '応用',
  },
} as const

export function docsPageArea(slug: DocsSlug): DocsArea {
  if (slug === 'components') return 'components'
  if (slug === 'swift-ui' || slug === 'swift-ui-hosting') return 'swift-ui'
  return 'guide'
}

export function docsNavGroups(
  locale: Locale,
  frontend: DocsFrontend,
  area: DocsArea,
): readonly DocsNavGroup[] {
  const text = labels[locale]
  function guide(slug: DocsSlug): DocsNavItem {
    const source = findDocsPage(frontend, slug)
    if (!source) throw new Error(`Unknown docs page: ${frontend}/${slug}`)
    const page = localizedDocsPage(source, locale)
    return {
      title: page.title,
      description: page.description,
      path: docsPath(frontend, page.slug),
      terms: page.searchTerms.join(' '),
    }
  }
  function component(item: ComponentDoc): DocsNavItem {
    return {
      title: item.name,
      description: localizedComponentDescription(item, locale),
      path: componentDocsPath(item, frontend),
      terms: `${item.section} ${item.parts.join(' ')} ${item.keyProps.join(' ')}`,
    }
  }

  if (area === 'components') {
    return [
      {
        id: 'overview',
        title: text.overview,
        items: [guide('components')],
      },
      ...COMPONENT_NAV_GROUPS.map((group): DocsNavGroup => ({
        id: 'components',
        title: text.components,
        items: group.items.map(component),
      })),
    ]
  }

  if (area === 'swift-ui') {
    return [
      {
        id: 'swift-ui',
        title: text.swiftUi,
        items: [guide('swift-ui'), guide('swift-ui-hosting')],
      },
      {
        id: 'swift-ui-components',
        title: text.swiftComponents,
        items: SWIFT_UI_NAV_GROUP.items.map(component),
      },
    ]
  }

  return [
    {
      id: 'introduction',
      title: text.introduction,
      items: [guide('getting-started'), guide('project-structure')],
    },
    {
      id: 'guides',
      title: text.guides,
      items: [
        guide('reactivity'),
        guide('rendering'),
        guide('styling'),
        guide('forms-and-input'),
        guide('overlays-and-dialogs'),
        guide('routing'),
        guide('animations'),
        guide('native-services'),
      ],
    },
    {
      id: 'advanced',
      title: text.advanced,
      items: [
        guide('app-icon'),
        guide('tray-icon'),
        guide('bundled-resources'),
        guide('updater'),
        guide('extensions'),
      ],
    },
  ]
}
