import type { en } from "./en";

export const ja: typeof en = {
  meta: {
    title: "QuickGUI — Go、TypeScript、Rust でネイティブデスクトップアプリを作る",
    description:
      "Go、TypeScript、Rust でネイティブデスクトップアプリを構築。高速ビルド、細粒度のリアクティビティ、GPU 描画、アクセシブルなコンポーネント。メモリとアプリサイズの実測値も公開。",
  },
  common: {
    skipToContent: "コンテンツへスキップ",
    getStarted: "はじめる",
    copy: "“{{text}}” をコピー",
    copied: "コピーしました",
    language: "言語",
    frontend: "アプリの言語",
    docsFor: "{{language}} ドキュメント",
  },
  nav: {
    features: "機能",
    code: "コード",
    quickstart: "クイックスタート",
    docs: "ドキュメント",
    benchmarks: "ベンチマーク",
  },
  hero: {
    badge: "Pre-alpha",
    badgeHint: "API は変更されます",
    titleLine1: "ネイティブデスクトップアプリを、",
    titleLine2: "Go、TypeScript、Rust で。",
    sub: "Go、TypeScript、Rust でネイティブ UI を構築。共通の GPU レンダラー、高速な増分ビルド、細粒度のリアクティビティ、アクセシブルなコンポーネントを利用できます。",
  },
  features: {
    title: "機能",
    items: {
      idle: {
        title: "変化がなければ休止",
        body: "画面に変化がない間、ウィンドウは休止します。シグナルは関連するプロパティだけを更新し、イベント内の変更は一度の再描画にまとめます。",
      },
      fast: {
        title: "高速な増分ビルド",
        body: "ネイティブランタイムを再利用し、Go、TypeScript、Rust のアプリを再コンパイル。待ち時間を減らし、UI の改善に集中できます。",
      },
      layout: {
        title: "Flexbox と CSS Grid",
        body: "Flexbox、CSS Grid、なじみのあるスタイルオプション。Go、TypeScript、Rust でレイアウトを組み立て、スタイルを再利用できます。",
      },
      components: {
        title: "コンポーネント同梱",
        body: "アクセシブルでスタイルなしのコンポーネントを多数同梱——メニュー、ダイアログ、ポップオーバー、select、combobox、タブ、テーブル、ツリー。自由に仕上げられます",
      },
      text: {
        title: "テキスト編集",
        body: "選択、編集、アンドゥ、IME、絵文字、右から左へ書く言語に対応しています。",
      },
      a11y: {
        title: "標準でアクセシブル",
        body: "スクリーンリーダーには本物のボタン・リスト・テキストが見えます。追加コードは不要",
      },
      lists: {
        title: "仮想化リスト",
        body: "仮想化テーブルはスクロールに合わせて表示中の行をマウントし、大きな履歴やデータビューを扱いやすくします。",
      },
      native: {
        title: "ネイティブのデスクトップ機能",
        body: "本物のウィンドウ、システムメニュー、ファイルダイアログ、トレイアイコン、通知をアプリで利用できます。",
      },
      cli: {
        title: "開発とパッケージ化",
        body: "quickgui dev は編集しながら実行、quickgui build は署名済みでインストール可能なリリースを出力します",
      },
    },
  },
  code: {
    title: "Go、TypeScript、Rust のコード例",
    lead: "同じリアクティブなカウンターを、3 つの言語で。言語を選んでコードを見てみましょう。",
    frontends: {
      go: "メソッドチェーンで組み立てるネイティブビューと再利用できるスタイル。シグナルが UI を同期します。",
      typescript: "Solid 2 のシグナルとなじみのある JSX で、ネイティブコンポーネントを描画します。",
      rust: "Rust のビルダーでネイティブビューを構築し、状態はビューに保持します。",
    },
  },
  swiftUi: {
    title: "QuickGUI でネイティブ SwiftUI を使う",
    lead: "macOS の Go、TypeScript、Rust アプリに本物の SwiftUI コントロールを埋め込み、QuickGUI コンポーネントと一緒に使えます。",
  },
  quickstart: {
    title: "クイックスタート",
    create: "作成して実行",
    edit: "整形とビルド",
    ship: "署名とパッケージ化",
  },
  platforms: {
    available: "利用可能",
    soon: "開発中",
  },
  cta: {
    title: "QuickGUI をはじめる",
    body: "Go、TypeScript、Rust のコンポーネントを書いて、ランタイムを同梱したネイティブアプリを配布しましょう。",
    docs: "ドキュメントを読む",
    star: "GitHub でスターする",
  },
  footer: {
    license: "MIT または Apache-2.0",
  },
  benchmarks: {
    title: "ベンチマーク",
    lead: "QuickGUI Go、QuickGUI TypeScript、QuickGUI Rust、GPUI、Tauri、Electron のアイドル時メモリとインストールサイズを同じ Mac で測定しました。各アプリで同じ課題管理ツールを動かし、1,000 件の課題を読み込みます。",
    memory: "アプリのアイドル時メモリ",
    bundle: "インストールサイズ",
    measured: "測定日：{{date}}",
    machine: "{{chip}} · メモリ {{ram}} GiB · macOS {{os}} · arm64",
    methodology: "測定方法",
    method:
      "1,100 × 720 のウィンドウに同じ 1,000 件の課題を読み込み、1 ページあたり 100 行を保持して最初の課題を選択。各フレームワークを {{runs}} 回起動します。少なくとも {{warmup}} 秒待ち、メモリが安定して CPU が 1% 以下の状態が 15 秒続いてから、さらに {{samples}} 回測定します。棒は各起動の中央値の中央値、ひげはその範囲です。",
    memoryMethod:
      "メモリはアプリと描画補助プロセスの物理フットプリントの合計です。WebKit や Chromium の描画、GPU、ネットワークプロセスを含み、自動入力などの macOS サービスは除外します。測定中も CPU とメモリの安定を確認し、スクリーンショットは測定後に撮影します。圧縮メモリを含み、アクティビティモニタと同じ十進 MB（1,000,000 バイト）で表示します。強制 GC やキャッシュ削除は行いません。",
    bundleMethod:
      "サイズは .app 内に配布するファイルを集計し、ネイティブライブラリやフレームワークを含みます。QuickGUI TypeScript は Bun と Solid 2 を同梱し、QuickGUI Rust はネイティブコアを実行ファイルにリンクし、GPUI は Zed の GPU UI フレームワークを実行ファイルに含め、Electron は Chromium と Node を同梱します。OS が提供する WebKit などは除外します。圧縮ダウンロードではなく、インストール後のサイズです。",
    scope:
      "検索、状態フィルター、ページ切り替え、メモの編集、完了操作に対応しています。データはセッション中のメモリに保持し、データベースやネットワークサービスは使いません。グラフは読み込み後のアイドル時の使用量で、操作の処理性能を表すものではありません。結果はアプリ、マシン、OS によって変わります。QuickGUI は開発版からビルドしており、各結果に使用したリビジョンを記録しています。",
    preview: "ベンチマーク用アプリ",
    previewAlt: "{{framework}} で 1,000 件の課題を表示するベンチマークアプリ",
    raw: "生の測定データ",
    source: "ベンチマークを再現",
    version: "バージョン {{version}}",
    tableCaption: "アイドル時メモリとインストールサイズの測定値",
    framework: "フレームワーク",
    unit: "MB",
    range: "起動ごとの中央値：{{min}}–{{max}} MB",
  },
};
