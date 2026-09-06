package host

import "testing"

func TestLibraryNameAndStage(t *testing.T) {
	if LibraryName() == "" || StageTarget() == "" {
		t.Fatal("empty host library identity")
	}
	if Current.ProtocolVersion() != 0 {
		t.Fatal("unloaded host should not report a protocol")
	}
}
