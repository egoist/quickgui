package ui

import "github.com/egoist/quickgui/go/native"

import (
	gui "github.com/egoist/quickgui/go/ui"
	"quickgui.example/quick-git/internal/git"
)

func commitRefBadge(ref git.CommitRef) *gui.Element {
	theme := UseApp().Theme()
	background, color := theme.AccentWash, theme.Accent
	if ref.Current {
		background, color = theme.Accent, theme.TextOnAccent
	} else if ref.Kind == "tag" {
		background, color = theme.WarningWash, theme.Warning
	}
	return gui.View().
		Child(

			gui.Text(
				ref.Name,
			).
				FontSize(10.5).
				FontWeight(700).
				TextColor(color).
				LineClamp(1).
				TextOverflow("ellipsis"),
		).
		Display("flex").
		Height(16).
		MinWidth(0).
		MaxWidth(140).
		AlignItems("center").
		PaddingLeft(5).
		PaddingRight(5).
		BorderRadius(4).
		BackgroundColor(background)
}

// Reserve space for the subject even when a commit has several long ref names.
func commitRefs(read func() []git.CommitRef) *native.Node {
	visible := gui.CreateMemo(func() []git.CommitRef {
		refs := make([]git.CommitRef, 0, 3)
		for _, ref := range read() {
			if ref.Kind == "head" {
				continue
			}
			refs = append(refs, ref)
			if len(refs) == 3 {
				break
			}
		}
		return refs
	})
	return gui.Show(
		len(visible()) > 0,
		func() *gui.Element {
			return gui.View().
				Child(

					gui.For(
						visible,
						func(ref git.CommitRef, _ func() int) *gui.Element {
							return commitRefBadge(ref)
						},
						func(ref git.CommitRef) any { return string(ref.Kind) + ":" + ref.Name },
						nil,
					),
				).
				Display("flex").
				FlexDirection("row").
				AlignItems("center").
				MinWidth(0).
				MaxWidth("48%").
				FlexShrink(1).
				Overflow("hidden").
				Gap(4)
		},
	)
}
