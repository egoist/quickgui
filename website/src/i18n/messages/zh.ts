import type { en } from "./en";

export const zh: typeof en = {
  meta: {
    title: "QuickGUI — 用 Go、TypeScript 或 Rust 构建原生桌面应用",
    description:
      "用 Go、TypeScript 或 Rust 构建原生桌面应用。快速构建、细粒度响应式、GPU 渲染和无障碍组件，并提供内存与应用体积的实测对比。",
  },
  common: {
    skipToContent: "跳到正文",
    getStarted: "开始使用",
    copy: "复制 “{{text}}”",
    copied: "已复制",
    language: "语言",
    frontend: "应用语言",
    docsFor: "{{language}} 文档",
  },
  nav: {
    features: "特性",
    code: "代码",
    quickstart: "快速开始",
    docs: "文档",
    benchmarks: "基准测试",
  },
  hero: {
    badge: "Pre-alpha",
    badgeHint: "接口可能变更",
    titleLine1: "构建原生桌面应用，",
    titleLine2: "用 Go、TypeScript 或 Rust。",
    sub: "使用 Go、TypeScript 或 Rust 开发原生界面，支持快速增量构建、细粒度响应式和无障碍组件，共享同一个 GPU 渲染器。",
  },
  features: {
    title: "特性",
    items: {
      idle: {
        title: "空闲窗口保持休眠",
        body: "界面没有变化时，窗口保持休眠。信号只更新相关属性，事件中的更新会合并为一次重绘。",
      },
      fast: {
        title: "快速增量构建",
        body: "重新编译 Go、TypeScript 或 Rust 应用时，复用原生运行时。缩短反馈周期，把更多时间留给界面打磨。",
      },
      layout: {
        title: "Flexbox 与 CSS Grid",
        body: "使用 Flexbox、CSS Grid 和熟悉的样式选项，在 Go、TypeScript 或 Rust 中组合布局并复用组件样式。",
      },
      components: {
        title: "组件开箱即用",
        body: "一大批可访问的无样式组件——菜单、对话框、弹出层、select、combobox、标签页、表格、树——随你定制",
      },
      text: {
        title: "文本编辑",
        body: "支持文本选择、编辑、撤销、输入法、emoji 和从右到左的文字。",
      },
      a11y: {
        title: "默认无障碍",
        body: "屏幕阅读器看到的是真实的按钮、列表和文本, 无需额外代码",
      },
      lists: {
        title: "虚拟化列表",
        body: "虚拟化表格随滚动挂载可见行，轻松管理较大的历史记录和数据视图。",
      },
      native: {
        title: "原生桌面能力",
        body: "真实窗口、系统菜单、文件对话框、托盘图标和通知，都是应用的一部分。",
      },
      cli: {
        title: "开发与打包",
        body: "quickgui dev 边改边跑;quickgui build 直接产出已签名、可安装的发布版本",
      },
    },
  },
  code: {
    title: "Go、TypeScript 与 Rust 示例",
    lead: "同一个响应式计数器，三种熟悉的语言。选择语言，查看对应代码。",
    frontends: {
      go: "链式原生视图、可组合样式与信号，让界面随状态同步更新。",
      typescript: "使用 Solid 2 信号与熟悉的 JSX，渲染真正的原生组件。",
      rust: "用 Rust 构建原生视图，状态保存在视图上。",
    },
  },
  swiftUi: {
    title: "在 QuickGUI 中使用原生 SwiftUI",
    lead: "在 macOS 的 Go、TypeScript 或 Rust 应用中嵌入真正的 SwiftUI 控件，与 QuickGUI 组件一起使用。",
  },
  quickstart: {
    title: "快速开始",
    create: "创建并运行",
    edit: "格式化与构建",
    ship: "签名与打包",
  },
  platforms: {
    available: "现已可用",
    soon: "开发中",
  },
  cta: {
    title: "开始使用 QuickGUI",
    body: "编写 Go、TypeScript 或 Rust 组件，发布自带运行时的原生应用。",
    docs: "阅读文档",
    star: "在 GitHub 加星",
  },
  footer: {
    license: "MIT 或 Apache-2.0",
  },
  benchmarks: {
    title: "基准测试",
    lead: "在同一台 Mac 上测量 QuickGUI Go、QuickGUI TypeScript、QuickGUI Rust、GPUI、Tauri 和 Electron 的空闲内存与安装体积。每个应用都运行相同的任务管理器，包含 1,000 条任务。",
    memory: "应用空闲内存",
    bundle: "安装体积",
    measured: "测量日期：{{date}}",
    machine: "{{chip}} · {{ram}} GiB 内存 · macOS {{os}} · arm64",
    methodology: "测量方法",
    method:
      "一个 1,100 × 720 窗口，加载相同的 1,000 条任务，每页保留 100 行，并选中第一条任务。每个框架重新启动 {{runs}} 次，每次至少等待 {{warmup}} 秒，并要求内存稳定、CPU 不超过 1% 的状态持续 15 秒，再进行 {{samples}} 次空闲采样。柱条表示各次启动中位数的中位数，端点线表示它们的范围。",
    memoryMethod:
      "内存为应用及渲染辅助进程的物理占用之和，包括 WebKit 或 Chromium 的渲染、GPU 和网络进程，不包含自动填充及其他 macOS 系统服务。采样期间 CPU 和内存必须保持稳定，截图在采样结束后进行。数据包含压缩内存，使用与活动监视器一致的十进制 MB（1,000,000 字节），不强制垃圾回收或清空缓存。",
    bundleMethod:
      "体积统计 .app 内实际分发的文件，包括原生库和框架。QuickGUI TypeScript 包含 Bun 和 Solid 2，QuickGUI Rust 将原生核心链接进可执行文件，GPUI 将 Zed 的 GPU UI 框架打进可执行文件，Electron 包含 Chromium 和 Node，不含系统提供的 WebKit 等框架。这是安装体积，而非压缩下载大小。",
    scope:
      "演示支持搜索、状态筛选、翻页、备注编辑和完成操作。数据仅在当前会话的内存中保存，不使用数据库或网络服务。图表衡量应用加载后的空闲占用，不代表交互吞吐量。结果会随应用、设备和操作系统变化。QuickGUI 使用开发版本构建，每项结果均记录了对应的代码版本。",
    preview: "基准测试应用",
    previewAlt: "{{framework}} 运行包含 1,000 条任务的基准演示",
    raw: "原始测量数据",
    source: "复现基准测试",
    version: "版本 {{version}}",
    tableCaption: "空闲内存与安装体积实测",
    framework: "框架",
    unit: "MB",
    range: "各次启动中位数：{{min}}–{{max}} MB",
  },
};
