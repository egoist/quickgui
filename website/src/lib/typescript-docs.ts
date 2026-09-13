import { DOCS_GUIDE_ORDER, docsOutline, docsTitle } from "./docs-structure";
import type { DocsPageMeta } from "./docs";

const descriptions = {
  "getting-started": "Create a native desktop app with TypeScript and open your first window.",
  "project-structure": "The files in a QuickGUI TypeScript project and how to configure them.",
  components: "Write components in JSX and use the built-in control set.",
  reactivity: "Keep UI in sync with Solid signals, memos, and effects.",
  rendering: "Show and hide content, render lists, and work with the current window.",
  styling: "Lay out and style views with Flexbox, Grid, and interaction states.",
  "forms-and-input": "Build text fields, checkboxes, radios, and select controls.",
  "overlays-and-dialogs": "Show popovers, dialogs, and operating-system file pickers.",
  routing: "Declare routes, nested layouts, and in-app navigation.",
  animations: "Animate hover, color, and opacity, and play GIF or WebP images.",
  "native-services": "Open windows and use menus, clipboard, dialogs, and file watching.",
  "swift-ui": "Embed real SwiftUI controls inside a TypeScript application.",
  "swift-ui-hosting": "Style SwiftUI controls and nest QuickGUI content inside them.",
  "app-icon": "Use resources/icon.png as the packaged application icon, or change the window or Dock icon at runtime.",
  "tray-icon": "Add a menu-bar or notification-area icon, and mark macOS template images by filename or flag.",
  "bundled-resources": "Ship files in resources/ and load them from the packaged resource directory.",
  updater: "Ship signed automatic updates for macOS, Windows, and Linux.",
  extensions: "Share Solid components or add a native service your app can call.",
} as const;

const zhDescriptions = {
  "getting-started": "用 TypeScript 创建原生桌面应用并打开第一个窗口。",
  "project-structure": "了解 QuickGUI TypeScript 项目中各个文件的用途。",
  components: "用 JSX 编写组件，并使用内置控件。",
  reactivity: "用 Solid 的信号、memo 和 effect 保持界面同步。",
  rendering: "显示和隐藏内容、渲染列表，并使用当前窗口。",
  styling: "用 Flexbox、网格和交互状态布局并设置样式。",
  "forms-and-input": "构建文本字段、复选框、单选框和选择器。",
  "overlays-and-dialogs": "显示浮层、对话框和操作系统文件选择器。",
  routing: "声明路由、嵌套布局和应用内导航。",
  animations: "为悬停、颜色和不透明度添加动画，并播放 GIF 或 WebP。",
  "native-services": "打开窗口，并使用菜单、剪贴板、对话框和文件监听。",
  "swift-ui": "在 TypeScript 应用中嵌入真正的 SwiftUI 控件。",
  "swift-ui-hosting": "设置 SwiftUI 控件样式，并在其中放入 QuickGUI 内容。",
  "app-icon": "用 resources/icon.png 作为打包后的应用图标，或在运行时更换窗口与程序坞图标。",
  "tray-icon": "添加菜单栏或通知区图标，并用文件名或标志把图像标为 macOS 模板图像。",
  "bundled-resources": "把文件放进 resources/，并从打包后的资源目录读取它们。",
  updater: "为 macOS、Windows 和 Linux 发布已签名的自动更新。",
  extensions: "共享 Solid 组件，或添加应用可调用的原生服务。",
} as const;

const jaDescriptions = {
  "getting-started": "TypeScript でネイティブデスクトップアプリを作成し、最初のウィンドウを開きます。",
  "project-structure": "QuickGUI の TypeScript プロジェクトの各ファイルの役割です。",
  components: "JSX でコンポーネントを書き、組み込みコントロールを使います。",
  reactivity: "Solid のシグナル、memo、effect で UI を同期します。",
  rendering: "内容の表示と非表示、リスト、現在のウィンドウを扱います。",
  styling: "Flexbox、グリッド、インタラクション状態でレイアウトとスタイルを設定します。",
  "forms-and-input": "テキストフィールド、チェックボックス、ラジオ、セレクトを作ります。",
  "overlays-and-dialogs": "ポップオーバー、ダイアログ、OS のファイルピッカーを表示します。",
  routing: "ルート、入れ子のレイアウト、アプリ内ナビゲーションを宣言します。",
  animations: "ホバー、色、不透明度をアニメーションし、GIF や WebP を再生します。",
  "native-services": "ウィンドウを開き、メニュー、クリップボード、ダイアログ、ファイル監視を使います。",
  "swift-ui": "TypeScript アプリに本物の SwiftUI コントロールを埋め込みます。",
  "swift-ui-hosting": "SwiftUI コントロールにスタイルを付け、その中に QuickGUI の内容を置きます。",
  "app-icon": "resources/icon.png をパッケージのアプリアイコンにし、実行時のウィンドウまたは Dock アイコンも変えられます。",
  "tray-icon": "メニューバーまたは通知領域のアイコンを追加し、ファイル名またはフラグで macOS のテンプレート画像にします。",
  "bundled-resources": "resources/ にファイルを置き、パッケージ後のリソースディレクトリから読み込みます。",
  updater: "macOS、Windows、Linux 向けの署名済み自動更新を配布します。",
  extensions: "Solid コンポーネントを共有するか、アプリから呼べるネイティブサービスを追加します。",
} as const;

export const TYPESCRIPT_DOCS_PAGES: readonly DocsPageMeta[] = DOCS_GUIDE_ORDER.map((slug) => ({
  frontend: "typescript",
  slug,
  title: docsTitle(slug),
  outline: docsOutline(slug),
  description: descriptions[slug],
  searchTerms: ["typescript", "bun", "solid", "jsx", slug],
  translations: {
    zh: { description: zhDescriptions[slug] },
    ja: { description: jaDescriptions[slug] },
  },
}));
