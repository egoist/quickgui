export const en = {
  meta: {
    title: "QuickGUI — Native desktop apps with Go, TypeScript, or Rust.",
    description:
      "Build native desktop apps with Go, TypeScript, or Rust. Fast builds, fine-grained reactivity, GPU rendering, and accessible components. Explore measured memory and bundle sizes.",
  },
  common: {
    skipToContent: "Skip to content",
    getStarted: "Get started",
    copy: "Copy “{{text}}”",
    copied: "Copied",
    language: "Language",
    frontend: "App language",
    docsFor: "{{language}} docs",
  },
  nav: {
    features: "Features",
    code: "Code",
    quickstart: "Quickstart",
    docs: "Docs",
    benchmarks: "Benchmarks",
  },
  hero: {
    badge: "Pre-alpha",
    badgeHint: "APIs may change",
    titleLine1: "Build native desktop apps.",
    titleLine2: "With Go, TypeScript, or Rust.",
    sub: "Build native interfaces in Go, TypeScript, or Rust with fast incremental builds, fine-grained reactivity, and accessible components powered by the same GPU renderer.",
  },
  features: {
    title: "Features",
    items: {
      idle: {
        title: "Idle windows stay idle",
        body: "Clean windows sleep until something changes. Signals update the affected properties, and event updates are batched into one redraw.",
      },
      fast: {
        title: "Fast incremental builds",
        body: "Recompile your Go, TypeScript, or Rust app while reusing the native runtime. Keep the feedback loop short as you refine your interface.",
      },
      layout: {
        title: "Flexbox and CSS Grid",
        body: "Flexbox and CSS Grid with familiar style options. Compose layouts in Go, TypeScript, or Rust and reuse styles across components.",
      },
      components: {
        title: "Components included",
        body: "A large collection of accessible, unstyled components — menus, dialogs, popovers, selects, comboboxes, tabs, tables, trees — ready to make yours.",
      },
      text: {
        title: "Text editing",
        body: "Support for selection, editing, undo, input methods, emoji, and right-to-left scripts.",
      },
      a11y: {
        title: "Accessible by default",
        body: "Screen readers see real buttons, lists, and text — no extra code required.",
      },
      lists: {
        title: "Virtualized lists",
        body: "Virtualized tables mount the visible rows as you scroll, keeping large histories and data views manageable.",
      },
      native: {
        title: "Native desktop features",
        body: "Real windows, system menus, file dialogs, tray icons, and notifications are part of your application.",
      },
      cli: {
        title: "Development and packaging",
        body: "quickgui dev runs your app and reloads as you edit; quickgui build hands you a signed, installable release.",
      },
    },
  },
  code: {
    title: "Go, TypeScript, and Rust examples",
    lead: "One reactive counter, three familiar languages. Pick a language to explore the code.",
    frontends: {
      go: "Fluent native views and composable styles, with signals that keep your UI in sync.",
      typescript: "Solid 2 signals and familiar JSX, rendered as native components.",
      rust: "Native views with Rust builders, and state that stays on the view.",
    },
  },
  swiftUi: {
    title: "Use native SwiftUI in QuickGUI.",
    lead: "Embed real SwiftUI controls in your Go, TypeScript, or Rust application on macOS, alongside your QuickGUI components.",
  },
  quickstart: {
    title: "Quickstart",
    create: "Create and run",
    edit: "Format and build",
    ship: "Sign and package",
  },
  platforms: {
    available: "available now",
    soon: "in progress",
  },
  cta: {
    title: "Get started with QuickGUI",
    body: "Write a Go, TypeScript, or Rust component and ship a native app with its runtime included.",
    docs: "Read the docs",
    star: "Star on GitHub",
  },
  footer: {
    license: "MIT or Apache-2.0",
  },
  benchmarks: {
    title: "Benchmarks",
    lead: "Idle memory and installed bundle size for QuickGUI Go, QuickGUI TypeScript, QuickGUI Rust, GPUI, Tauri, and Electron, measured on the same Mac. Each app runs the same issue tracker with 1,000 issues.",
    memory: "Idle app memory",
    bundle: "Installed bundle",
    measured: "Measured {{date}}",
    machine: "{{chip}} · {{ram}} GiB RAM · macOS {{os}} · arm64",
    methodology: "How we measured",
    method:
      "One 1,100 × 720 window with the same 1,000 issues, 100 retained rows per page, and the first issue selected. {{runs}} fresh launches per framework. Wait at least {{warmup}} seconds and require 15 seconds of stable memory and CPU at or below 1%, then take {{samples}} further idle samples. Bars show the median of the launch medians; whiskers show their range.",
    memoryMethod:
      "Memory is the sum of physical footprints for the app and its rendering helpers, including WebKit or Chromium renderer, GPU, and network processes. AutoFill and other macOS services are excluded. CPU and memory must stay stable throughout sampling; screenshots are taken afterward. Values include compressed memory and use decimal MB (1,000,000 bytes), as in Activity Monitor. No forced garbage collection or cache purge.",
    bundleMethod:
      "Bundle size counts the files shipped inside the .app, including native libraries and frameworks. QuickGUI TypeScript includes Bun and Solid 2, QuickGUI Rust links the native core into its executable, GPUI ships Zed's GPU UI framework in its executable, and Electron includes Chromium and Node. OS-provided frameworks such as WebKit are excluded. This is installed size, not a compressed download.",
    scope:
      "The demo supports search, status filters, pagination, editable notes, and completion actions. Data stays in memory for the session, with no database or network service. These charts measure the loaded app at idle, not interaction throughput. Results vary with your app, machine, and operating system. QuickGUI results use development checkouts; each result records its exact revision.",
    preview: "Benchmark apps",
    previewAlt: "{{framework}} running the issue tracker benchmark with 1,000 issues",
    raw: "Raw measurements",
    source: "Reproduce the benchmark",
    version: "Version {{version}}",
    tableCaption: "Measured idle memory and installed bundle size",
    framework: "Framework",
    unit: "MB",
    range: "Launch medians: {{min}}–{{max}} MB",
  },
};
