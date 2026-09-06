package protocol

import (
	"encoding/binary"
	"fmt"
	"math"
)

const (
	opCreateElement  = 1
	opCreateText     = 2
	opCreateSentinel = 3
	opSetProperty    = 4
	opReplaceText    = 5
	opInsert         = 6
	opRemove         = 7
	opCleanup        = 8

	propClear  = 0
	propBool   = 1
	propNumber = 2
	propColor  = 3
	propString = 4
)

// Batch encodes one transactional mutation list for the host.
type Batch struct {
	bytes  []byte
	length int
	count  int
}

// NewBatch allocates an empty mutation batch with room for the 10-byte header.
func NewBatch() *Batch {
	return &Batch{bytes: make([]byte, 256), length: 10}
}

func (b *Batch) Empty() bool { return b.count == 0 }

func (b *Batch) MutationCount() int { return b.count }

func (b *Batch) CreateElement(id uint32, tag uint8) {
	b.op(opCreateElement)
	b.u32(id)
	b.u8(tag)
}

func (b *Batch) CreateText(id uint32, value string) {
	b.op(opCreateText)
	b.u32(id)
	b.string(value)
}

func (b *Batch) CreateSentinel(id uint32) {
	b.op(opCreateSentinel)
	b.u32(id)
}

func (b *Batch) ClearProperty(id uint32, property uint16) {
	b.op(opSetProperty)
	b.u32(id)
	b.u16(property)
	b.u8(propClear)
}

func (b *Batch) SetBoolean(id uint32, property uint16, value bool) {
	b.op(opSetProperty)
	b.u32(id)
	b.u16(property)
	b.u8(propBool)
	if value {
		b.u8(1)
	} else {
		b.u8(0)
	}
}

func (b *Batch) SetNumber(id uint32, property uint16, value float32) {
	if math.IsNaN(float64(value)) || math.IsInf(float64(value), 0) {
		panic(fmt.Sprintf("QuickGUI property %d must be finite", property))
	}
	b.op(opSetProperty)
	b.u32(id)
	b.u16(property)
	b.u8(propNumber)
	b.f32(value)
}

func (b *Batch) SetColor(id uint32, property uint16, value uint32) {
	b.op(opSetProperty)
	b.u32(id)
	b.u16(property)
	b.u8(propColor)
	b.u32(value)
}

func (b *Batch) SetString(id uint32, property uint16, value string) {
	b.op(opSetProperty)
	b.u32(id)
	b.u16(property)
	b.u8(propString)
	b.string(value)
}

func (b *Batch) ReplaceText(id uint32, value string) {
	b.op(opReplaceText)
	b.u32(id)
	b.string(value)
}

func (b *Batch) Insert(parent, child, before uint32) {
	b.op(opInsert)
	b.u32(parent)
	b.u32(child)
	b.u32(before)
}

func (b *Batch) Remove(parent, child uint32) {
	b.op(opRemove)
	b.u32(parent)
	b.u32(child)
}

func (b *Batch) Cleanup(parent uint32, children []uint32) {
	b.op(opCleanup)
	b.u32(parent)
	b.u32(uint32(len(children)))
	for _, child := range children {
		b.u32(child)
	}
}

// Append splices another detached subtree's mutations, preserving order.
func (b *Batch) Append(other *Batch) {
	body := other.Body()
	b.ensure(len(body))
	copy(b.bytes[b.length:], body)
	b.length += len(body)
	b.count += other.count
}

// Body is the encoded mutations without the header.
func (b *Batch) Body() []byte {
	out := make([]byte, b.length-10)
	copy(out, b.bytes[10:b.length])
	return out
}

// Finish returns the complete encoded batch, or nil when nothing was recorded.
func (b *Batch) Finish() []byte {
	if b.count == 0 {
		return nil
	}
	b.bytes[0] = 0x51
	b.bytes[1] = 0x47
	b.bytes[2] = 0x4d
	b.bytes[3] = 0x42
	binary.LittleEndian.PutUint16(b.bytes[4:], uint16(Version))
	binary.LittleEndian.PutUint32(b.bytes[6:], uint32(b.count))
	out := make([]byte, b.length)
	copy(out, b.bytes[:b.length])
	return out
}

func (b *Batch) op(opcode byte) {
	b.count++
	b.u8(opcode)
}

func (b *Batch) ensure(additional int) {
	required := b.length + additional
	if required <= len(b.bytes) {
		return
	}
	capacity := len(b.bytes)
	for capacity < required {
		capacity *= 2
	}
	next := make([]byte, capacity)
	copy(next, b.bytes[:b.length])
	b.bytes = next
}

func (b *Batch) u8(value byte) {
	b.ensure(1)
	b.bytes[b.length] = value
	b.length++
}

func (b *Batch) u16(value uint16) {
	b.ensure(2)
	binary.LittleEndian.PutUint16(b.bytes[b.length:], value)
	b.length += 2
}

func (b *Batch) u32(value uint32) {
	b.ensure(4)
	binary.LittleEndian.PutUint32(b.bytes[b.length:], value)
	b.length += 4
}

func (b *Batch) f32(value float32) {
	b.ensure(4)
	binary.LittleEndian.PutUint32(b.bytes[b.length:], math.Float32bits(value))
	b.length += 4
}

func (b *Batch) string(value string) {
	raw := []byte(value)
	b.u32(uint32(len(raw)))
	b.ensure(len(raw))
	copy(b.bytes[b.length:], raw)
	b.length += len(raw)
}
