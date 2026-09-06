package ui

import "testing"

func TestRepositoryLabels(t *testing.T) {
	labels := RepositoryLabels([]string{"/work/app", "/personal/app", "/tmp/other"})
	if labels["/work/app"] != "work/app" || labels["/tmp/other"] != "other" {
		t.Fatalf("%v", labels)
	}
}
