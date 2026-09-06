package model

import (
	"path/filepath"

	"github.com/egoist/quickgui/examples/quick-git-go/internal/git"
)

func CanonicalPath(path string) string {
	return git.CanonicalPath(path)
}

func Basename(path string) string {
	return filepath.Base(path)
}

func Dirname(path string) string {
	return filepath.Dir(path)
}
