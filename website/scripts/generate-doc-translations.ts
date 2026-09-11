import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { ALL_COMPONENT_DOCS } from '../src/lib/component-docs'
import {
  COMPONENT_DOC_LABELS,
  componentDescriptionTranslations,
} from '../src/lib/docs-locales'

type TranslatedLocale = 'zh' | 'ja'

const translations: Record<
  string,
  Record<TranslatedLocale, string>
> = {

  'Unix timestamp in milliseconds; `Min` and `Max` use the same unit.': {
    zh: '以毫秒表示的 Unix 时间戳；`Min` 和 `Max` 使用相同单位。',
    ja: 'ミリ秒単位の Unix タイムスタンプです。`Min` と `Max` も同じ単位です。',
  },
  '`automatic`, `field`, `stepperField`, or `graphical`.': {
    zh: '`automatic`、`field`、`stepperField` 或 `graphical`。',
    ja: '`automatic`、`field`、`stepperField`、`graphical` のいずれかです。',
  },
  'Native SwiftUI presentation style for this control.': {
    zh: '此控件的原生 SwiftUI 展示样式。',
    ja: 'このコントロールのネイティブ SwiftUI 表示スタイルです。',
  },

  'Static route declarations built with `ui.Route` and `ui.Layout`.': {
    zh: '使用 `ui.Route` 和 `ui.Layout` 创建的静态路由声明。',
    ja: '`ui.Route` と `ui.Layout` で作成する静的ルート宣言です。',
  },
  'Absolute or parent-relative route pattern.': {
    zh: '绝对路由模式或相对于父路由的模式。',
    ja: '絶対または親からの相対ルートパターンです。',
  },
  'Accessibility role exposed by the retained node.': {
    zh: '保留节点暴露的无障碍角色。',
    ja: '保持ノードが公開するアクセシビリティロールです。',
  },
  'Accessible name for a component whose visible parts may change.': {
    zh: '当组件的可见部件可能变化时使用的无障碍名称。',
    ja: '表示パーツが変化するコンポーネントに付けるアクセシブル名です。',
  },
  'Adds an AM/PM segment and uses a twelve-hour clock.': {
    zh: '添加上午/下午分段，并使用 12 小时制。',
    ja: '午前／午後セグメントを追加し、12 時間表記を使います。',
  },
  'Alias for the image fit policy.': {
    zh: '图像适配策略的别名。',
    ja: '画像のフィット方針の別名です。',
  },
  'Allowed drawer extents, expressed as viewport fractions or pixels.': {
    zh: '抽屉可用的展开位置，以视口比例或像素表示。',
    ja: 'ドロワーで許可する展開量を、ビューポート比率またはピクセルで指定します。',
  },
  'Allows alpha selection in the native color panel.': {
    zh: '允许在原生颜色面板中选择 Alpha 值。',
    ja: 'ネイティブのカラーパネルでアルファ値を選択できるようにします。',
  },
  'Allows Escape to dismiss the surface.': {
    zh: '允许按 Escape 键关闭界面。',
    ja: 'Escape キーでサーフェスを閉じられるようにします。',
  },
  'Allows more than one value to be selected or opened.': {
    zh: '允许选择或打开多个值。',
    ja: '複数の値を選択または開けるようにします。',
  },
  'Allows single or multiple table selection.': {
    zh: '允许表格单选或多选。',
    ja: 'テーブルで単一選択または複数選択を有効にします。',
  },
  'Allows the pointer to cross into the tooltip without closing it.': {
    zh: '允许指针移入 Tooltip 而不会将其关闭。',
    ja: '閉じずにポインターを Tooltip 内へ移動できるようにします。',
  },
  'Anchor point on the composed SwiftUI trigger.': {
    zh: '组合式 SwiftUI 触发器上的锚点。',
    ja: '組み合わせた SwiftUI トリガー上のアンカーポイントです。',
  },
  'Arguments passed to the terminal process.': {
    zh: '传递给终端进程的参数。',
    ja: 'ターミナルプロセスに渡す引数です。',
  },
  'Bounded declaration model used for options or keyboard navigation.': {
    zh: '用于选项或键盘导航的有限声明模型。',
    ja: '選択肢やキーボードナビゲーションに使う、上限付きの宣言モデルです。',
  },
  'Bounded hierarchical node declarations for the tree.': {
    zh: '树使用的有限层级节点声明。',
    ja: 'Tree の上限付き階層ノード宣言です。',
  },
  'Bounded message exposed to error content and accessibility.': {
    zh: '向错误内容和无障碍 API 暴露的有限长度消息。',
    ja: 'エラー内容とアクセシビリティに公開する、長さ制限付きメッセージです。',
  },
  'Chooses date, time, or combined date-and-time fields.': {
    zh: '选择日期、时间或日期和时间组合字段。',
    ja: '日付、時刻、または日付と時刻の両方のフィールドを選びます。',
  },
  'Chooses ordinary scrolling or tail-following behavior.': {
    zh: '选择普通滚动或跟随末尾模式。',
    ja: '通常のスクロールまたは末尾追従の動作を選びます。',
  },
  'Chooses the single- or multiple-selection policy.': {
    zh: '选择单选或多选策略。',
    ja: '単一選択または複数選択の方針を選びます。',
  },
  'Clear color for the reverse-hosted QuickGUI WGPU surface.': {
    zh: '反向托管 QuickGUI WGPU 表面的清除颜色。',
    ja: 'リバースホストした QuickGUI WGPU サーフェスのクリアカラーです。',
  },
  'Column declarations for the virtual table.': {
    zh: '虚拟表格的列声明。',
    ja: '仮想テーブルの列宣言です。',
  },
  'Complete declared toast queue rendered by the viewport.': {
    zh: '视口渲染的完整 Toast 队列声明。',
    ja: 'ビューポートが描画する、宣言済み Toast キュー全体です。',
  },
  'Complete ordered value universe used to derive a parent checkbox state.': {
    zh: '用于派生父复选框状态的完整有序值集合。',
    ja: '親チェックボックスの状態導出に使う、完全で順序付きの値集合です。',
  },
  'Component rendered when a route wins.': {
    zh: '路由匹配时渲染的组件。',
    ja: 'ルートが一致したときに描画するコンポーネントです。',
  },
  'Contains focus and blocks interaction behind the surface.': {
    zh: '将焦点限制在界面内，并阻止与后方内容交互。',
    ja: 'フォーカスを内部に閉じ込め、背後のサーフェスへの操作を防ぎます。',
  },
  'Content mounted inside the component.': {
    zh: '挂载在组件内部的内容。',
    ja: 'コンポーネント内にマウントする内容です。',
  },
  'Content rendered when no route matches.': {
    zh: '没有路由匹配时渲染的内容。',
    ja: 'どのルートにも一致しないときに描画する内容です。',
  },
  'Controlled checked state.': {
    zh: '受控的选中状态。',
    ja: '制御されたチェック状態です。',
  },
  'Controlled expanded tree node identifiers.': {
    zh: '受控的已展开树节点标识符。',
    ja: '制御された展開済み Tree ノード ID です。',
  },
  'Controlled item holding the active or roving focus state.': {
    zh: '持有当前活动状态或移动焦点状态的受控项目。',
    ja: 'アクティブ状態またはロービングフォーカス状態を持つ、制御された項目です。',
  },
  'Controlled native SwiftUI popover presentation state.': {
    zh: '受控的原生 SwiftUI Popover 显示状态。',
    ja: '制御されたネイティブ SwiftUI Popover の表示状態です。',
  },
  'Controlled open state.': {
    zh: '受控的打开状态。',
    ja: '制御された開閉状態です。',
  },
  'Controlled pressed state.': {
    zh: '受控的按下状态。',
    ja: '制御された押下状態です。',
  },
  'Controlled selection for a table, native picker, or color control.': {
    zh: '表格、原生选择器或颜色控件的受控选中值。',
    ja: 'テーブル、ネイティブピッカー、カラーコントロールの制御選択値です。',
  },
  'Controlled SwiftUI toggle value.': {
    zh: '受控的 SwiftUI 开关值。',
    ja: '制御された SwiftUI Toggle の値です。',
  },
  'Controlled value. The precise value type follows this component’s contract.': {
    zh: '受控值，具体类型取决于该组件的约定。',
    ja: '制御値です。具体的な型は、このコンポーネントの規約に従います。',
  },
  'Core-owned contains, prefix, fuzzy, or disabled filtering.': {
    zh: '由核心管理的包含、前缀、模糊或禁用筛选模式。',
    ja: 'コアが管理する、部分一致、前方一致、あいまい一致、または無効の絞り込みです。',
  },
  'Core-owned delay in milliseconds before closing.': {
    zh: '关闭前由核心管理的延迟（毫秒）。',
    ja: '閉じるまでの、コアが管理する遅延時間（ミリ秒）です。',
  },
  'Core-owned delay in milliseconds before opening.': {
    zh: '打开前由核心管理的延迟（毫秒）。',
    ja: '開くまでの、コアが管理する遅延時間（ミリ秒）です。',
  },
  'Cursor color or color accessor.': {
    zh: '光标颜色或颜色访问器。',
    ja: 'カーソル色または色のアクセサー。',
  },
  'Declared logical height.': {
    zh: '声明的逻辑高度。',
    ja: '宣言された論理高さです。',
  },
  'Declared logical width.': {
    zh: '声明的逻辑宽度。',
    ja: '宣言された論理幅です。',
  },
  'Declared scroll content extent.': {
    zh: '声明的滚动内容范围。',
    ja: '宣言されたスクロール内容の範囲です。',
  },
  'Declared scroll viewport extent.': {
    zh: '声明的滚动视口范围。',
    ja: '宣言されたスクロールビューポートの範囲です。',
  },
  'Denominator for determinate SwiftUI progress.': {
    zh: 'SwiftUI 确定进度的分母。',
    ja: '確定型の SwiftUI 進捗で使う分母です。',
  },
  'Destination resolved by the QuickGUI router.': {
    zh: 'QuickGUI 路由器解析的目标地址。',
    ja: 'QuickGUI Router が解決する移動先です。',
  },
  'Distance from an edge that still counts as being at that edge.': {
    zh: '仍视为位于边缘的最大距离。',
    ja: 'その端にいるとみなす最大距離です。',
  },
  'Edge toward which a swipe dismisses or moves the surface.': {
    zh: '滑动关闭或移动界面时朝向的边缘。',
    ja: 'スワイプでサーフェスを閉じる、または移動する方向の端です。',
  },
  'Editable query text used by a combobox or autocomplete.': {
    zh: 'Combobox 或 Autocomplete 使用的可编辑查询文本。',
    ja: 'Combobox または Autocomplete が使う、編集可能な検索テキストです。',
  },
  'Enables multiline text editing.': {
    zh: '启用多行文本编辑。',
    ja: '複数行テキスト編集を有効にします。',
  },
  'Enables native font thickening.': {
    zh: '启用原生字体加粗处理。',
    ja: 'ネイティブのフォント増厚を有効にします。',
  },
  'Environment entries added to the terminal process.': {
    zh: '添加到终端进程的环境变量。',
    ja: 'ターミナルプロセスに追加する環境変数です。',
  },
  'Executable launched by the terminal.': {
    zh: '终端启动的可执行文件。',
    ja: 'ターミナルが起動する実行ファイルです。',
  },
  'Extra rows mounted before and after the visible range.': {
    zh: '在可见范围前后额外挂载的行数。',
    ja: '表示範囲の前後に追加でマウントする行数です。',
  },
  'Filesystem path, data URL, SVG document, or shader source expected by the component.': {
    zh: '组件所需的文件系统路径、数据 URL、SVG 文档或着色器源码。',
    ja: 'コンポーネントが受け取るファイルパス、データ URL、SVG ドキュメント、またはシェーダーソースです。',
  },
  'Filesystem path, file URL, or base64 data URL for an avatar image.': {
    zh: '头像图像的文件系统路径、文件 URL 或 base64 数据 URL。',
    ja: 'アバター画像のファイルパス、ファイル URL、または base64 データ URL です。',
  },
  'First weekday column, where Monday is 0 and Sunday is 6.': {
    zh: '第一列对应的星期；周一为 0，周日为 6。',
    ja: '最初の曜日列です。月曜日が 0、日曜日が 6 です。',
  },
  'Horizontal or vertical layout and keyboard direction.': {
    zh: '水平或垂直布局，以及对应的键盘方向。',
    ja: '水平または垂直のレイアウトと、それに対応するキーボード方向です。',
  },
  'How image content fits its declared box.': {
    zh: '图像内容在声明区域中的适配方式。',
    ja: '宣言したボックスに画像内容を収める方法です。',
  },
  'Human-readable value preferred by assistive technology.': {
    zh: '辅助技术优先使用的易读值。',
    ja: '支援技術向けの読みやすい値です。',
  },
  'Increment used by keyboard, pointer, or native control input.': {
    zh: '键盘、指针或原生控件输入使用的步长。',
    ja: 'キーボード、ポインター、またはネイティブコントロール入力で使う増分です。',
  },
  'Inherited auto-dismiss duration or warm-provider timeout.': {
    zh: '继承的自动关闭时长或预热 Provider 的超时时间。',
    ja: '継承する自動消去時間、またはウォームアップ済み Provider のタイムアウトです。',
  },
  'Initial active item for an uncontrolled component.': {
    zh: '非受控组件的初始活动项目。',
    ja: '非制御コンポーネントの初期アクティブ項目です。',
  },
  'Initial checked state when the component owns it.': {
    zh: '组件自行管理状态时的初始选中状态。',
    ja: 'コンポーネント自身が状態を管理するときの初期チェック状態です。',
  },
  'Initial destination for the bounded in-memory history.': {
    zh: '有界内存历史的初始目标地址。',
    ja: '上限付きメモリ履歴の初期移動先です。',
  },
  'Initial logical row-height estimate used by virtualization.': {
    zh: '虚拟化使用的初始逻辑行高估计值。',
    ja: '仮想化で使う初期論理行高の推定値です。',
  },
  'Initial pressed state when the component owns it.': {
    zh: '组件自行管理状态时的初始按下状态。',
    ja: 'コンポーネント自身が状態を管理するときの初期押下状態です。',
  },
  'Initial state when the component owns whether it is open.': {
    zh: '组件自行管理打开状态时的初始状态。',
    ja: 'コンポーネント自身が開閉状態を管理するときの初期状態です。',
  },
  'Initial value when the component owns its state.': {
    zh: '组件自行管理状态时的初始值。',
    ja: 'コンポーネント自身が状態を管理するときの初期値です。',
  },
  'Initial working directory for the terminal process.': {
    zh: '终端进程的初始工作目录。',
    ja: 'ターミナルプロセスの初期作業ディレクトリです。',
  },
  'Keeps closing content mounted for the exact exit transition duration.': {
    zh: '在精确的退出过渡时长内保持关闭中的内容已挂载。',
    ja: '閉じる内容を、正確な終了トランジション時間だけマウントしたままにします。',
  },
  'Keeps inactive content mounted with display disabled.': {
    zh: '保持非活动内容已挂载，但禁用其显示。',
    ja: '非アクティブな内容をマウントしたまま、表示を無効にします。',
  },
  'Logical height of each virtualized row.': {
    zh: '每个虚拟化行的逻辑高度。',
    ja: '仮想化された各行の論理高さです。',
  },
  'Lower boundary of the high meter range.': {
    zh: '仪表高值区间的下界。',
    ja: 'Meter の高い範囲の下限です。',
  },
  'Markdown source to shape and paint.': {
    zh: '用于排版和绘制的 Markdown 源文本。',
    ja: '組版・描画する Markdown ソースです。',
  },
  'Marks the value as required for validation and form submission.': {
    zh: '将值标记为验证和表单提交所必需。',
    ja: '検証とフォーム送信で必須の値としてマークします。',
  },
  'Maximum accepted value.': {
    zh: '可接受的最大值。',
    ja: '受け入れる最大値です。',
  },
  'Maximum retained scrollback lines.': {
    zh: '保留的最大回滚行数。',
    ja: '保持するスクロールバックの最大行数。',
  },
  'Minimum accepted value.': {
    zh: '可接受的最小值。',
    ja: '受け入れる最小値です。',
  },
  'Mounts a seconds segment.': {
    zh: '挂载秒数分段。',
    ja: '秒セグメントをマウントします。',
  },
  'Number of declared menubar items.': {
    zh: '已声明的菜单栏项目数。',
    ja: '宣言されたメニューバー項目の数です。',
  },
  'Number of one-time-code input slots, bounded to twelve.': {
    zh: '一次性验证码输入格数量，上限为 12。',
    ja: 'ワンタイムコード入力スロットの数です。上限は 12 です。',
  },
  'Ordered native picker options.': {
    zh: '有序的原生选择器选项。',
    ja: '順序付きのネイティブピッカー選択肢です。',
  },
  'Ordered SwiftUI view modifiers applied to the hosted control.': {
    zh: '应用于托管控件的有序 SwiftUI View 修饰器。',
    ja: 'ホストしたコントロールへ順番に適用する SwiftUI View モディファイアです。',
  },
  'Per-pane minimum size and collapse declarations.': {
    zh: '每个面板的最小尺寸和折叠声明。',
    ja: 'ペインごとの最小サイズと折りたたみ宣言です。',
  },
  'Pins short list content to the top or bottom.': {
    zh: '将较短的列表内容固定在顶部或底部。',
    ja: '短いリスト内容を上端または下端に固定します。',
  },
  'Preferred alignment on the placement cross-axis.': {
    zh: '在放置交叉轴上的首选对齐方式。',
    ja: '配置の交差軸上で優先する揃え方です。',
  },
  'Preferred edge for the native SwiftUI popover arrow.': {
    zh: '原生 SwiftUI Popover 箭头的首选边缘。',
    ja: 'ネイティブ SwiftUI Popover の矢印を置く優先辺です。',
  },
  'Preferred placement side before collision handling.': {
    zh: '碰撞处理前的首选放置方向。',
    ja: '衝突処理前に優先する配置方向です。',
  },
  'Preferred point or range for the meter value.': {
    zh: '仪表值的理想点或区间。',
    ja: 'Meter 値の望ましい点または範囲です。',
  },
  'Prevents interaction and exposes the disabled state.': {
    zh: '禁止交互，并暴露禁用状态。',
    ja: '操作を無効にし、無効状態を公開します。',
  },
  'Prevents value changes while leaving the control focusable.': {
    zh: '阻止值变化，但仍允许控件获得焦点。',
    ja: '値の変更を防ぎつつ、コントロールはフォーカス可能なままにします。',
  },
  'Projects invalid state to the field and its parts.': {
    zh: '将无效状态传递给字段及其各部件。',
    ja: 'Field とそのパーツへ検証エラー状態を反映します。',
  },
  'QuickGUI layout, paint, typography, and interaction-state styles.': {
    zh: 'QuickGUI 的布局、绘制、排版和交互状态样式。',
    ja: 'QuickGUI のレイアウト、描画、タイポグラフィ、インタラクション状態のスタイルです。',
  },
  'Readable label for the current native progress or gauge value.': {
    zh: '当前原生进度或仪表值的易读标签。',
    ja: '現在のネイティブ進捗または Gauge 値を表す読みやすいラベルです。',
  },
  'Receives the retained native node.': {
    zh: '接收保留式原生节点。',
    ja: '保持されたネイティブノードを受け取ります。',
  },
  'Reports a committed native value change.': {
    zh: '报告已提交的原生值变化。',
    ja: '確定したネイティブ値の変更を通知します。',
  },
  'Reports an open or close decision from the core.': {
    zh: '报告核心做出的打开或关闭决定。',
    ja: 'コアが決定した開閉を通知します。',
  },
  'Reports clamped offset and derived overflow state.': {
    zh: '报告限制后的偏移量和派生的溢出状态。',
    ja: '制限適用後のオフセットと、そこから得たオーバーフロー状態を通知します。',
  },
  'Reports every avatar image load-state transition.': {
    zh: '报告头像图像每次加载状态变化。',
    ja: 'アバター画像の読み込み状態が変わるたびに通知します。',
  },
  'Reports native editing input.': {
    zh: '报告原生编辑输入。',
    ja: 'ネイティブ編集の入力を通知します。',
  },
  'Reports native SwiftUI popover presentation changes.': {
    zh: '报告原生 SwiftUI Popover 的显示状态变化。',
    ja: 'ネイティブ SwiftUI Popover の表示状態変更を通知します。',
  },
  'Reports one explicit commit boundary and its final value details.': {
    zh: '报告一次明确的提交边界及最终值详情。',
    ja: '明示的なコミット境界と最終値の詳細を通知します。',
  },
  'Reports terminal process lifecycle changes.': {
    zh: '报告终端进程的生命周期变化。',
    ja: 'ターミナルプロセスのライフサイクル変更を通知します。',
  },
  'Reports the checked state accepted by the core.': {
    zh: '报告核心接受的选中状态。',
    ja: 'コアが受け入れたチェック状態を通知します。',
  },
  'Reports the complete pane-size set after the core resizes it.': {
    zh: '在核心调整大小后报告完整的面板尺寸集合。',
    ja: 'コアによるサイズ変更後、完全なペインサイズ集合を通知します。',
  },
  'Reports the derived status, formatted value, and completion ratio.': {
    zh: '报告派生状态、格式化值和完成比例。',
    ja: '導出された状態、整形済みの値、完了率を通知します。',
  },
  'Reports the editable query text retained by the core.': {
    zh: '报告核心保留的可编辑查询文本。',
    ja: 'コアが保持した編集可能な検索テキストを通知します。',
  },
  'Reports the item that now owns the roving focus state.': {
    zh: '报告当前持有移动焦点状态的项目。',
    ja: 'ロービングフォーカス状態を現在持つ項目を通知します。',
  },
  'Reports the menu entry selected by the core.': {
    zh: '报告核心选中的菜单项。',
    ja: 'コアが選択したメニュー項目を通知します。',
  },
  'Reports the ordered node identifiers expanded by the core.': {
    zh: '报告核心展开的有序节点标识符。',
    ja: 'コアが展開した順序付きノード ID を通知します。',
  },
  'Reports the pressed state accepted by the core.': {
    zh: '报告核心接受的按下状态。',
    ja: 'コアが受け入れた押下状態を通知します。',
  },
  'Reports the selected native picker option or color.': {
    zh: '报告选中的原生选择器选项或颜色。',
    ja: '選択したネイティブピッカー項目または色を通知します。',
  },
  'Reports the validation trigger and timing selected by the core.': {
    zh: '报告核心选择的验证触发条件和时机。',
    ja: 'コアが選んだ検証のトリガーとタイミングを通知します。',
  },
  'Reports the value accepted by the core.': {
    zh: '报告核心接受的值。',
    ja: 'コアが受け入れた値を通知します。',
  },
  'Reports the value accepted by the native SwiftUI toggle.': {
    zh: '报告原生 SwiftUI 开关接受的值。',
    ja: 'ネイティブ SwiftUI Toggle が受け入れた値を通知します。',
  },
  'Reports toast identifiers dismissed by the core.': {
    zh: '报告核心关闭的 Toast 标识符。',
    ja: 'コアが閉じた Toast ID を通知します。',
  },
  'Represents progress whose completed amount is unknown.': {
    zh: '表示完成量未知的进度。',
    ja: '完了量が不明な進捗を表します。',
  },
  'Requested native child-window placement.': {
    zh: '请求的原生子窗口放置位置。',
    ja: 'ネイティブ子ウィンドウの配置要求です。',
  },
  'Runs after the core resolves an activation.': {
    zh: '在核心完成激活判定后运行。',
    ja: 'コアがアクティブ化を解決した後に実行します。',
  },
  'Runs when the native field submits its current value.': {
    zh: '原生字段提交当前值时运行。',
    ja: 'ネイティブフィールドが現在値を送信したときに実行します。',
  },
  'Runs when the native SwiftUI button is activated.': {
    zh: '原生 SwiftUI 按钮激活时运行。',
    ja: 'ネイティブ SwiftUI Button がアクティブになったときに実行します。',
  },
  'Selects which interaction boundary triggers validation.': {
    zh: '选择触发验证的交互边界。',
    ja: '検証を開始するインタラクション境界を選びます。',
  },
  'Sixteen ANSI colors (`terminal.Palette`) or an accessor. Updates preserve the PTY.': {
    zh: '16 色 ANSI 调色板（`terminal.Palette`）或访问器，更新时保留 PTY。',
    ja: '16 色の ANSI パレット（`terminal.Palette`）またはアクセサー。更新しても PTY は維持されます。',
  },
  'SF Symbols name displayed with a SwiftUI button label.': {
    zh: '与 SwiftUI 按钮标签一起显示的 SF Symbols 名称。',
    ja: 'SwiftUI Button のラベルとともに表示する SF Symbols 名です。',
  },
  'Stable queue identifier associated with one rendered toast.': {
    zh: '与一个已渲染 Toast 关联的稳定队列标识符。',
    ja: '描画された 1 つの Toast に対応する、安定したキュー ID です。',
  },
  'Structural colors and geometry for a core-painted native menu or picker.': {
    zh: '核心绘制的原生菜单或选择器所用的结构颜色和几何属性。',
    ja: 'コア描画のネイティブメニューまたはピッカーに使う構造色とジオメトリです。',
  },
  'SwiftUI semantic role: `default`, `cancel`, or `destructive`.': {
    zh: 'SwiftUI 语义角色：`default`、`cancel` 或 `destructive`。',
    ja: 'SwiftUI のセマンティックロールです：`default`、`cancel`、`destructive`。',
  },
  'Text shown while the editable value is empty.': {
    zh: '可编辑值为空时显示的文本。',
    ja: '編集値が空のときに表示するテキストです。',
  },
  'Total logical row count.': {
    zh: '逻辑总行数。',
    ja: '論理上の総行数です。',
  },
  'Up to sixteen numeric values packed into fixed shader parameter vectors.': {
    zh: '最多 16 个数值，打包到固定的着色器参数向量中。',
    ja: '固定のシェーダーパラメーターベクトルへ格納する最大 16 個の数値です。',
  },
  'Upper boundary of the low meter range.': {
    zh: '仪表低值区间的上界。',
    ja: 'Meter の低い範囲の上限です。',
  },
  'Use `"extend"` to paint padding with terminal edge backgrounds.': {
    zh: '使用 `"extend"` 将终端边缘背景延伸到内边距。',
    ja: '`"extend"` で端の背景色を余白まで延長します。',
  },
  'Uses incremental Markdown reconciliation for appended content.': {
    zh: '对追加内容使用增量 Markdown 协调。',
    ja: '追記された内容に Markdown の差分更新を使います。',
  },
  'Uses the hosted subtree’s intrinsic width, height, or both.': {
    zh: '使用托管子树的固有宽度、高度或两者。',
    ja: 'ホストしたサブツリー固有の幅、高さ、またはその両方を使います。',
  },
  'Visible native SwiftUI control label.': {
    zh: '原生 SwiftUI 控件的可见标签。',
    ja: 'ネイティブ SwiftUI コントロールに表示するラベルです。',
  },
  'Whether focus or an explicit press activates a tab.': {
    zh: '焦点或明确按下操作是否会激活标签页。',
    ja: 'フォーカスまたは明示的な押下で Tab をアクティブにするかどうかです。',
  },
  'Wraps highlight navigation at the ends of a menu level.': {
    zh: '到达菜单层级末端时循环高亮导航。',
    ja: 'メニューレベルの端でハイライト移動を循環させます。',
  },
  'Wraps keyboard highlight at the ends of the current menu level.': {
    zh: '到达当前菜单层级末端时循环键盘高亮。',
    ja: '現在のメニューレベルの端でキーボードハイライトを循環させます。',
  },
}

const websiteRoot = resolve(import.meta.dir, '..')
const locales = ['zh', 'ja'] as const
// Go component locales are hand-tuned. This generator writes Rust locales from English Rust MDX.
const frontends = ['rust'] as const

function translateSource(
  source: string,
  locale: TranslatedLocale,
  description: string,
): string {
  const labels = COMPONENT_DOC_LABELS[locale]
  let inFence = false
  let replacedDescription = false

  const lines = source.split('\n').map((line) => {
    if (line.startsWith('```')) {
      inFence = !inFence
      return line
    }
    if (inFence) return line

    if (!replacedDescription && line.trim()) {
      replacedDescription = true
      return description
    }

    if (line === '## Import') return `## ${labels.import}`
    if (line === '## Usage') return `## ${labels.usage}`
    if (line === '## Anatomy') return `## ${labels.anatomy}`
    if (line === '## Key props') return `## ${labels.keyProps}`
    if (line === '| Prop | Purpose |') {
      return `| ${labels.prop} | ${labels.purpose} |`
    }

    const importMatch = line.match(/^Import (`[^`]+`) from (`[^`]+`)\.$/)
    if (importMatch) {
      return locale === 'zh'
        ? `从 ${importMatch[2]} 导入 ${importMatch[1]}。`
        : `${importMatch[2]} から ${importMatch[1]} をインポートします。`
    }

    const propMatch = line.match(/^\| (`[^`]+`) \| (.+) \|$/)
    if (propMatch) {
      const translated = translations[propMatch[2]]?.[locale]
      if (!translated) {
        throw new Error(`Missing ${locale} prop translation: ${propMatch[2]}`)
      }
      return `| ${propMatch[1]} | ${translated} |`
    }

    return line
  })

  if (inFence) throw new Error('Unclosed code fence')
  return lines.join('\n')
}

let written = 0
for (const frontend of frontends) {
  const componentRoot = resolve(websiteRoot, `src/content/docs/${frontend}/components`)
  for (const locale of locales) {
    const descriptions = componentDescriptionTranslations(locale)
    for (const component of ALL_COMPONENT_DOCS) {
      const key = `${component.kind}/${component.slug}`
      const description = descriptions[key]
      if (!description) throw new Error(`Missing ${locale} description: ${key}`)

      const family = component.kind === 'swift-ui' ? 'swift-ui' : 'ui'
      const sourcePath = resolve(componentRoot, family, `${component.slug}.mdx`)
      const destination = resolve(
        componentRoot,
        locale,
        family,
        `${component.slug}.mdx`,
      )
      const source = await readFile(sourcePath, 'utf8')
      const translated = translateSource(source, locale, description)
      await mkdir(resolve(componentRoot, locale, family), { recursive: true })
      await writeFile(destination, translated)
      written += 1
    }
  }
}

console.log(`Generated ${written} localized component MDX files`)
