package native

import "testing"

func TestParseColor(t *testing.T) {
	if ParseColor("#112233") != PackColor(0x11, 0x22, 0x33, 255) {
		t.Fatal("hex 6")
	}
	if ParseColor("#123") != PackColor(0x11, 0x22, 0x33, 255) {
		t.Fatal("hex 3")
	}
	if ParseColor("black") != PackColor(0, 0, 0, 255) {
		t.Fatal("black")
	}
	if ParseColor("transparent") != 0 {
		t.Fatal("transparent")
	}
	if ParseColor("rgb(1, 2, 3)") != PackColor(1, 2, 3, 255) {
		t.Fatal("rgb")
	}
}
