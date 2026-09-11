import GithubSlugger from 'github-slugger'
import type { Locale } from '../i18n'
import type { DocsOutlineItem, DocsSlug } from './docs'

// Shared guide order and sections keep every language frontend navigable in the
// same way. Their MDX bodies contain the frontend-specific APIs and examples.
export const DOCS_GUIDE_ORDER = [
  'getting-started',
  'project-structure',
  'updater',
  'extensions',
  'native-services',
  'reactivity',
  'rendering',
  'components',
  'routing',
  'styling',
  'animations',
  'forms-and-input',
  'overlays-and-dialogs',
  'swift-ui',
  'swift-ui-hosting',
] as const

const sectionTitles = {
  'getting-started': {
    en: ['Requirements', 'Create a project', 'Your first window', 'Run and build', 'Next steps'],
    zh: ['系统要求', '创建项目', '第一个窗口', '运行与构建', '后续步骤'],
    ja: ['動作要件', 'プロジェクトを作成', '最初のウィンドウ', '実行とビルド', '次のステップ'],
  },
  'project-structure': {
    en: ['Generated files', 'Application entry', 'Configuration', 'Packages', 'Local development'],
    zh: ['生成的文件', '应用入口', '配置', '软件包', '本地开发'],
    ja: ['生成されるファイル', 'アプリケーションのエントリ', '設定', 'パッケージ', 'ローカル開発'],
  },
  updater: {
    en: [
      'Configuration',
      'Application usage',
      'Lifecycle',
      'Platforms',
      'Publishing',
      'Native artifacts',
    ],
    zh: ['配置', '应用用法', '生命周期', '平台支持', '发布更新', '原生产物'],
    ja: [
      '設定',
      'アプリでの使用',
      'ライフサイクル',
      '対応プラットフォーム',
      '更新の公開',
      'ネイティブ成果物',
    ],
  },
  extensions: {
    en: [
      'Choose an extension type',
      'Create an extension project',
      'Share a component',
      'Lay out a native extension',
      'Declare dependencies',
      'Implement the native contract',
      'Automatic registration',
      'Build and package artifacts',
      'Test and distribute',
    ],
    zh: [
      '选择扩展类型',
      '创建扩展项目',
      '共享组件',
      '组织原生扩展',
      '声明依赖',
      '实现原生接口',
      '自动注册',
      '构建与打包产物',
      '测试与分发',
    ],
    ja: [
      '拡張の種類を選ぶ',
      '拡張プロジェクトを作成する',
      'コンポーネントの共有',
      'ネイティブ拡張の構成',
      '依存関係の宣言',
      'ネイティブインターフェースを実装する',
      '自動登録',
      '成果物のビルドとパッケージ化',
      'テストと配布',
    ],
  },
  'native-services': {
    en: ['Window lifetime', 'Commands and services', 'Events and file watching'],
    zh: ['窗口生命周期', '命令与服务', '事件与文件监听'],
    ja: ['ウィンドウのライフタイム', 'コマンドとサービス', 'イベントとファイル監視'],
  },
  reactivity: {
    en: ['Reactive state', 'Derived state', 'Effects and cleanup', 'Batched updates'],
    zh: ['响应式状态', '派生状态', '副作用与清理', '批量更新'],
    ja: ['リアクティブな状態', '派生状態', '副作用とクリーンアップ', '更新のバッチ処理'],
  },
  rendering: {
    en: [
      'Rendering model',
      'Conditional content',
      'Lists and identity',
      'Window lifecycle',
      'Current window',
      'Checking, testing, and debugging',
    ],
    zh: ['渲染模型', '条件内容', '列表与节点身份', '窗口生命周期', '当前窗口', '检查、测试与调试'],
    ja: [
      'レンダリングモデル',
      '条件付きコンテンツ',
      'リストとノードの同一性',
      'ウィンドウのライフサイクル',
      '現在のウィンドウ',
      'チェック・テスト・デバッグ',
    ],
  },
  components: {
    en: [
      'Defining components',
      'Primitives',
      'Compound components',
      'Controlled state',
      'Component families',
      'Styling parts',
    ],
    zh: ['定义组件', '基础组件', '复合组件', '受控状态', '组件类别', '设置各部件的样式'],
    ja: [
      'コンポーネントの定義',
      'プリミティブ',
      '複合コンポーネント',
      '制御された状態',
      'コンポーネントの種類',
      '各パーツのスタイル',
    ],
  },
  routing: {
    en: ['Routes', 'Nested layouts', 'Navigation', 'Route parameters'],
    zh: ['路由声明', '嵌套布局', '导航', '路由参数'],
    ja: ['ルートの宣言', '入れ子のレイアウト', 'ナビゲーション', 'ルートパラメーター'],
  },
  styling: {
    en: [
      'Style modifiers',
      'Merging styles',
      'Reusable styles',
      'Flexbox',
      'Grid',
      'Text and color',
      'Interaction states',
      'Groups and named group hover',
    ],
    zh: [
      '样式方法',
      '合并样式',
      '复用样式',
      'Flexbox',
      '网格布局',
      '文本与颜色',
      '交互状态',
      '分组与具名分组悬停',
    ],
    ja: [
      'スタイルメソッド',
      'スタイルのマージ',
      'スタイルの再利用',
      'Flexbox',
      'グリッド',
      'テキストと色',
      'インタラクション状態',
      'グループと名前付きグループのホバー',
    ],
  },
  animations: {
    en: [
      'Hover transitions',
      'State-driven animation',
      'Timing and frame rate',
      'Supported properties',
      'Mounting and reduced motion',
      'Animated images',
    ],
    zh: [
      '悬停过渡',
      '状态驱动的动画',
      '时长与帧率',
      '支持的属性',
      '挂载与减少动态效果',
      '动态图像',
    ],
    ja: [
      'ホバーのトランジション',
      '状態によるアニメーション',
      '時間とフレームレート',
      '対応するプロパティ',
      'マウントと視差効果の抑制',
      'アニメーション画像',
    ],
  },
  'forms-and-input': {
    en: ['Text input', 'Fields', 'Choices', 'Select', 'Events'],
    zh: ['文本输入', '字段', '选项', '选择器', '事件'],
    ja: ['テキスト入力', 'フィールド', '選択コントロール', 'Select', 'イベント'],
  },
  'overlays-and-dialogs': {
    en: ['Popover', 'Dialog', 'System popover', 'Native dialogs'],
    zh: ['Popover', 'Dialog', '系统浮窗', '原生对话框'],
    ja: ['Popover', 'Dialog', 'システムポップオーバー', 'ネイティブダイアログ'],
  },
  'swift-ui': {
    en: ['Host', 'Native controls', 'Controlled values', 'Sizing', 'Platform support'],
    zh: ['Host', '原生控件', '受控值', '尺寸', '平台支持'],
    ja: ['Host', 'ネイティブコントロール', '制御値', 'サイズ', '対応プラットフォーム'],
  },
  'swift-ui-hosting': {
    en: ['Modifiers', 'Liquid Glass', 'Reverse hosting', 'SwiftUI popover', 'Lifetime'],
    zh: ['修饰器', 'Liquid Glass', '反向托管', 'SwiftUI Popover', '生命周期'],
    ja: [
      'モディファイア',
      'Liquid Glass',
      'リバースホスティング',
      'SwiftUI ポップオーバー',
      'ライフサイクル',
    ],
  },
} satisfies Record<DocsSlug, Record<Locale, readonly string[]>>

export function docsOutline(slug: DocsSlug, locale: Locale = 'en'): readonly DocsOutlineItem[] {
  const slugger = new GithubSlugger()
  return sectionTitles[slug][locale].map((title) => ({ id: slugger.slug(title), title }))
}

const guideTitles = {
  'getting-started': {
    en: 'Getting Started',
    ja: 'はじめに',
    zh: '入门',
  },
  'project-structure': {
    en: 'Project Structure',
    ja: 'プロジェクト構成',
    zh: '项目结构',
  },
  updater: {
    en: 'Auto Updater',
    ja: '自動更新',
    zh: '自动更新',
  },
  extensions: {
    en: 'Authoring Extensions',
    ja: '拡張の作成',
    zh: '编写扩展',
  },
  'native-services': {
    en: 'Windows & Native Services',
    ja: 'ウィンドウとネイティブサービス',
    zh: '窗口与原生服务',
  },
  reactivity: {
    en: 'Reactivity',
    ja: 'リアクティビティ',
    zh: '响应式',
  },
  rendering: {
    en: 'Rendering',
    ja: 'レンダリング',
    zh: '渲染',
  },
  components: {
    en: 'Components',
    ja: 'コンポーネント',
    zh: '组件',
  },
  routing: {
    en: 'Routing',
    ja: 'ルーティング',
    zh: '路由',
  },
  styling: {
    en: 'Styling & Layout',
    ja: 'スタイルとレイアウト',
    zh: '样式与布局',
  },
  animations: {
    en: 'Transitions & Animation',
    ja: 'トランジションとアニメーション',
    zh: '过渡与动画',
  },
  'forms-and-input': {
    en: 'Forms & Input',
    ja: 'フォームと入力',
    zh: '表单与输入',
  },
  'overlays-and-dialogs': {
    en: 'Overlays & Dialogs',
    ja: 'オーバーレイとダイアログ',
    zh: '浮层与对话框',
  },
  'swift-ui': {
    en: 'SwiftUI',
    ja: 'SwiftUI',
    zh: 'SwiftUI',
  },
  'swift-ui-hosting': {
    en: 'Modifiers & Hosting',
    ja: 'モディファイアとホスティング',
    zh: '修饰器与托管',
  },
} satisfies Record<DocsSlug, Record<Locale, string>>

export function docsTitle(slug: DocsSlug, locale: Locale = 'en'): string {
  return guideTitles[slug][locale]
}
