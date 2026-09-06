package protocol

import (
	"encoding/binary"
	"math"
	"testing"
)

func TestFinishEmpty(t *testing.T) {
	if NewBatch().Finish() != nil {
		t.Fatal("empty batch should finish as nil")
	}
}

func TestCreateElementAndHeader(t *testing.T) {
	b := NewBatch()
	b.CreateElement(1, TagView)
	bytes := b.Finish()
	if len(bytes) < 10 {
		t.Fatalf("batch too short: %d", len(bytes))
	}
	if bytes[0] != 0x51 || bytes[1] != 0x47 || bytes[2] != 0x4d || bytes[3] != 0x42 {
		t.Fatalf("bad magic: %x", bytes[:4])
	}
	if binary.LittleEndian.Uint16(bytes[4:]) != uint16(Version) {
		t.Fatalf("protocol version %d", binary.LittleEndian.Uint16(bytes[4:]))
	}
	if binary.LittleEndian.Uint32(bytes[6:]) != 1 {
		t.Fatalf("mutation count %d", binary.LittleEndian.Uint32(bytes[6:]))
	}
	if bytes[10] != opCreateElement {
		t.Fatalf("opcode %d", bytes[10])
	}
	if binary.LittleEndian.Uint32(bytes[11:]) != 1 {
		t.Fatalf("id %d", binary.LittleEndian.Uint32(bytes[11:]))
	}
	if bytes[15] != TagView {
		t.Fatalf("tag %d", bytes[15])
	}
}

func TestSetNumberAndString(t *testing.T) {
	b := NewBatch()
	b.SetNumber(3, Width, 12.5)
	b.SetString(3, Display, "flex")
	bytes := b.Finish()
	if binary.LittleEndian.Uint32(bytes[6:]) != 2 {
		t.Fatalf("count %d", binary.LittleEndian.Uint32(bytes[6:]))
	}
	if bytes[10] != opSetProperty || bytes[17] != propNumber {
		t.Fatalf("first op malformed")
	}
	bits := binary.LittleEndian.Uint32(bytes[18:])
	if math.Float32frombits(bits) != 12.5 {
		t.Fatalf("number %v", math.Float32frombits(bits))
	}
}

func TestAppendPreservesOrder(t *testing.T) {
	parent := NewBatch()
	parent.CreateElement(1, TagView)
	child := NewBatch()
	child.CreateText(2, "hi")
	parent.Append(child)
	parent.Insert(1, 2, NoAnchor)
	if parent.MutationCount() != 3 {
		t.Fatalf("count %d", parent.MutationCount())
	}
	body := parent.Body()
	if body[0] != opCreateElement || body[6] != opCreateText {
		t.Fatalf("order %v", body)
	}
}
