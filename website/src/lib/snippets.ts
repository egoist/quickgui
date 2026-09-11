export const snippets = {
  typescriptSwiftUi: {
    lang: "tsx",
    code: `<Host matchContents>
  <Button
    label="Save changes"
    systemImage="checkmark"
    modifiers={[buttonStyle("glass")]}
  />
</Host>`,
  },
  rustSwiftUi: {
    lang: "rust",
    code: `let mut host = MacSwiftUiHost::new(|_id| {})?;
host.sync(&[SwiftUiButton::new(1)
    .label("Save changes")
    .system_image("checkmark")
    .style(SwiftUiButtonStyle::Glass)
    .into()])?;
native_view(host.view())
`,
  },
  typescriptCounter: {
    lang: "tsx",
    code: `function Counter() {
  const [count, setCount] = createSignal(0);
  return (
    <View flex-col size-full items-center justify-center gap-5
      background-color="#090d16"
      color="#e2e8f0">
      <Text>Count: {count()}</Text>
      <Button p-3 rounded-lg
        background-color="#2563eb"
        onClick={() => setCount(count() + 1)}>
        Increment
      </Button>
    </View>
  );
}
`,
  },
  typescriptCliCheck: { lang: "bash", code: "bun run check\nbun run test\nbun run fmt" },
  rustCounter: {
    lang: "rust",
    code: `impl View for Counter {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let increment = cx.listener("increment", |this, cx: &mut EventContext| {
            this.count += 1;
            cx.invalidate();
        });
        div()
            .flex_col()
            .size_full()
            .items_center()
            .justify_center()
            .gap_5()
            .bg(Color::rgb8(9, 13, 22))
            .text_color(Color::rgb8(226, 232, 240))
            .child(text(format!("Count: {}", self.count)))
            .child(
                button()
                    .p(12.0)
                    .rounded_lg()
                    .bg(Color::rgb8(37, 99, 235))
                    .on_click(increment)
                    .child("Increment"),
            )
    }
}
`,
  },
  counter: {
    lang: "go",
    code: `func Counter() *ui.Element {
	count, setCount := ui.CreateSignal(0)
	return ui.View(
		ui.Text("Count: ", count()),
		ui.Button("Increment").
			OnClick(func() { setCount(count() + 1) }).
			Padding(12).
			RoundedLg().
			Bg("#2563eb"),
	).FlexCol().
		SizeFull().
		ItemsCenter().
		JustifyCenter().
		Gap(20).
		Bg("#090d16").
		TextColor("#e2e8f0")
}
`,
  },
  window: {
    lang: "go",
    code: `package main

import (
	"log"

	"github.com/egoist/quickgui/go/native"
)

func main() {
	if err := native.Run(func() {
		openWindow()
		native.App.OnReopen(func(event native.ReopenEvent) {
			if !event.HasVisibleWindows {
				openWindow()
			}
		})
	}); err != nil {
		log.Fatal(err)
	}
}

func openWindow() {
	native.NewWindow(native.WindowOptions{
		Title:     "Counter",
		Width:     720,
		Height:    480,
		Component: Counter,
	})
}
`,
  },
  swiftUi: {
    lang: "go",
    code: `ui.SwiftUI.Host(
	ui.SwiftUIHostProps{MatchContents: true},
	func() *native.Node {
		return ui.SwiftUI.Button(ui.SwiftUIButtonProps{
			Label:       "Save changes",
			SystemImage: "checkmark",
			Modifiers: []ui.SwiftUIModifier{
				ui.SwiftUI.ButtonStyle("glass"),
			},
		})
	},
)
`,
  },
  cliInit: {
    lang: "bash",
    code: `bunx @quickgui/cli init my-app
cd my-app
bun run dev`,
  },
  cliFormat: {
    lang: "bash",
    code: `bun run fmt
bun run build`,
  },
  cliBuild: {
    lang: "bash",
    code: `bun run build --target darwin-arm64 \\
  --sign "Developer ID Application: Example (TEAMID)" \\
  --notarize quickgui-notary`,
  },
} as const;

export type SnippetKey = keyof typeof snippets;
export type HighlightedSnippets = Record<SnippetKey, string>;
