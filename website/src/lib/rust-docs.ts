import { DOCS_GUIDE_ORDER, docsOutline, docsTitle } from "./docs-structure";
import type { DocsPageMeta } from "./docs";

const descriptions = {
  "getting-started": "Build native desktop apps that link the QuickGUI crate with Cargo.",
  "project-structure": "Configure Rust projects, Cargo features, and native packaging.",
  updater: "Enable the updater crate feature and check signed update manifests.",
  extensions: "Share Rust modules in-app, or author native extensions for Go and TypeScript.",
  "native-services": "Manage windows, menus, dialogs, clipboard, and asynchronous native work.",
  reactivity: "Store view state, invalidate on change, and rebuild only the declared tree.",
  rendering: "Understand View::render, keyed identity, window ownership, and testing.",
  components: "Compose the complete QuickGUI component families with Rust builders.",
  routing: "Match nested native layouts with Rust-owned route matching and history.",
  styling: "Apply layout, typography, paint, and interaction styles to native elements.",
  animations: "Use native transitions and animated images without a frame-polling loop.",
  "forms-and-input": "Build controlled text inputs, fields, choices, and pickers.",
  "overlays-and-dialogs": "Compose popovers, dialogs, native panels, and file dialogs.",
  "swift-ui": "Embed real SwiftUI controls inside the retained QuickGUI renderer.",
  "swift-ui-hosting": "Style native controls and host QuickGUI content inside SwiftUI.",
};

export const RUST_DOCS_PAGES: readonly DocsPageMeta[] = DOCS_GUIDE_ORDER.map((slug) => ({
  frontend: "rust",
  slug,
  title: docsTitle(slug),
  outline: docsOutline(slug),
  description: descriptions[slug],
  searchTerms: ["rust", "cargo", "crate", "view", "wgpu", slug],
  translations: {
    zh: { description: `使用 Rust 与 Cargo：${docsTitle(slug, "zh")}。` },
    ja: { description: `Rust と Cargo での${docsTitle(slug, "ja")}。` },
  },
}));
