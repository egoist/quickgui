import { DOCS_GUIDE_ORDER, docsOutline, docsTitle } from "./docs-structure";
import type { DocsPageMeta } from "./docs";

const descriptions = {
  "getting-started": "Create a native desktop app with Rust and open your first window.",
  "project-structure": "The files in a QuickGUI Rust project and how to configure them.",
  components: "Write views, compose children, and use the built-in control set.",
  reactivity: "Store view state and redraw when it changes.",
  rendering: "Show and hide content, render lists, and work with the current window.",
  styling: "Lay out and style views with Flexbox, Grid, and interaction states.",
  "forms-and-input": "Build text fields, checkboxes, radios, and select controls.",
  "overlays-and-dialogs": "Show popovers, dialogs, and operating-system file pickers.",
  routing: "Declare routes, nested layouts, and in-app navigation.",
  animations: "Animate hover, color, and opacity, and play GIF or WebP images.",
  "native-services": "Open windows and use menus, clipboard, dialogs, and file watching.",
  "swift-ui": "Embed real SwiftUI controls inside a Rust application.",
  "swift-ui-hosting": "Style SwiftUI controls and nest QuickGUI content inside them.",
  "app-icon": "Set the packaged application icon and the window or Dock icon at runtime.",
  "bundled-resources": "Ship extra files with the app and load them from the resource directory.",
  updater: "Ship signed automatic updates for macOS, Windows, and Linux.",
  extensions: "Share Rust modules or author a native service for Go and TypeScript apps.",
} as const;

const zhDescriptions = {
  "getting-started": "用 Rust 创建原生桌面应用并打开第一个窗口。",
  "project-structure": "了解 QuickGUI Rust 项目中各个文件的用途。",
  components: "编写视图、组合子节点，并使用内置控件。",
  reactivity: "把状态保存在视图上，变更后重绘。",
  rendering: "显示和隐藏内容、渲染列表，并使用当前窗口。",
  styling: "用 Flexbox、网格和交互状态布局并设置样式。",
  "forms-and-input": "构建文本字段、复选框、单选框和选择器。",
  "overlays-and-dialogs": "显示浮层、对话框和操作系统文件选择器。",
  routing: "声明路由、嵌套布局和应用内导航。",
  animations: "为悬停、颜色和不透明度添加动画，并播放 GIF 或 WebP。",
  "native-services": "打开窗口，并使用菜单、剪贴板、对话框和文件监听。",
  "swift-ui": "在 Rust 应用中嵌入真正的 SwiftUI 控件。",
  "swift-ui-hosting": "设置 SwiftUI 控件样式，并在其中放入 QuickGUI 内容。",
  "app-icon": "设置打包后的应用图标，以及运行时的窗口或程序坞图标。",
  "bundled-resources": "把额外文件打进应用，并从资源目录读取它们。",
  updater: "为 macOS、Windows 和 Linux 发布已签名的自动更新。",
  extensions: "共享 Rust 模块，或为 Go 和 TypeScript 应用编写原生服务。",
} as const;

const jaDescriptions = {
  "getting-started": "Rust でネイティブデスクトップアプリを作成し、最初のウィンドウを開きます。",
  "project-structure": "QuickGUI の Rust プロジェクトの各ファイルの役割です。",
  components: "ビューを書き、子を組み合わせ、組み込みコントロールを使います。",
  reactivity: "ビューに状態を保持し、変化したら再描画します。",
  rendering: "内容の表示と非表示、リスト、現在のウィンドウを扱います。",
  styling: "Flexbox、グリッド、インタラクション状態でレイアウトとスタイルを設定します。",
  "forms-and-input": "テキストフィールド、チェックボックス、ラジオ、セレクトを作ります。",
  "overlays-and-dialogs": "ポップオーバー、ダイアログ、OS のファイルピッカーを表示します。",
  routing: "ルート、入れ子のレイアウト、アプリ内ナビゲーションを宣言します。",
  animations: "ホバー、色、不透明度をアニメーションし、GIF や WebP を再生します。",
  "native-services": "ウィンドウを開き、メニュー、クリップボード、ダイアログ、ファイル監視を使います。",
  "swift-ui": "Rust アプリに本物の SwiftUI コントロールを埋め込みます。",
  "swift-ui-hosting": "SwiftUI コントロールにスタイルを付け、その中に QuickGUI の内容を置きます。",
  "app-icon": "パッケージのアプリアイコンと、実行時のウィンドウまたは Dock アイコンを設定します。",
  "bundled-resources": "追加ファイルをアプリに同梱し、リソースディレクトリから読み込みます。",
  updater: "macOS、Windows、Linux 向けの署名済み自動更新を配布します。",
  extensions: "Rust モジュールを共有するか、Go と TypeScript アプリ向けのネイティブサービスを作ります。",
} as const;

export const RUST_DOCS_PAGES: readonly DocsPageMeta[] = DOCS_GUIDE_ORDER.map((slug) => ({
  frontend: "rust",
  slug,
  title: docsTitle(slug),
  outline: docsOutline(slug),
  description: descriptions[slug],
  searchTerms: ["rust", "cargo", "crate", "view", slug],
  translations: {
    zh: { description: zhDescriptions[slug] },
    ja: { description: jaDescriptions[slug] },
  },
}));
