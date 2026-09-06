package native

import (
	"fmt"
	"strconv"
	"strings"
)

// PackColor packs RGBA bytes in the host's little-endian layout.
func PackColor(r, g, b, a int) uint32 {
	component := func(value int) uint32 {
		if value < 0 {
			value = 0
		}
		if value > 255 {
			value = 255
		}
		return uint32(value)
	}
	return component(r) | (component(g) << 8) | (component(b) << 16) | (component(a) << 24)
}

// ParseColor accepts a packed RGBA integer, #rgb[a], #rrggbb[aa], rgb(), rgba(), or a keyword.
func ParseColor(value any) uint32 {
	switch color := value.(type) {
	case uint32:
		return color
	case int:
		return uint32(color)
	case float64:
		return uint32(color)
	case string:
		return parseColorString(color)
	default:
		panic(fmt.Sprintf("unsupported QuickGUI color %v", value))
	}
}

func parseColorString(value string) uint32 {
	color := strings.TrimSpace(strings.ToLower(value))
	if color == "transparent" {
		return 0
	}
	if color == "black" {
		return PackColor(0, 0, 0, 255)
	}
	if color == "white" {
		return PackColor(255, 255, 255, 255)
	}
	if strings.HasPrefix(color, "#") {
		hex := color[1:]
		if len(hex) == 3 || len(hex) == 4 {
			r := hexByte(hex, 0, true)
			g := hexByte(hex, 1, true)
			b := hexByte(hex, 2, true)
			a := 255
			if len(hex) == 4 {
				a = hexByte(hex, 3, true)
			}
			if r >= 0 && g >= 0 && b >= 0 && a >= 0 {
				return PackColor(r, g, b, a)
			}
		} else if len(hex) == 6 || len(hex) == 8 {
			r := hexByte(hex, 0, false)
			g := hexByte(hex, 2, false)
			b := hexByte(hex, 4, false)
			a := 255
			if len(hex) == 8 {
				a = hexByte(hex, 6, false)
			}
			if r >= 0 && g >= 0 && b >= 0 && a >= 0 {
				return PackColor(r, g, b, a)
			}
		}
	} else if (strings.HasPrefix(color, "rgba(") || strings.HasPrefix(color, "rgb(")) && strings.HasSuffix(color, ")") {
		inner := color[strings.Index(color, "(")+1 : len(color)-1]
		parts := strings.Split(inner, ",")
		if len(parts) == 3 || len(parts) == 4 {
			r, errR := strconv.ParseFloat(strings.TrimSpace(parts[0]), 64)
			g, errG := strconv.ParseFloat(strings.TrimSpace(parts[1]), 64)
			b, errB := strconv.ParseFloat(strings.TrimSpace(parts[2]), 64)
			a := 255.0
			var errA error
			if len(parts) == 4 {
				a, errA = strconv.ParseFloat(strings.TrimSpace(parts[3]), 64)
				a = a * 255
			}
			if errR == nil && errG == nil && errB == nil && errA == nil {
				return PackColor(int(r), int(g), int(b), int(a+0.5))
			}
		}
	}
	panic(fmt.Sprintf("unsupported QuickGUI color `%s`", value))
}

func hexDigit(code byte) int {
	if code >= '0' && code <= '9' {
		return int(code - '0')
	}
	if code >= 'a' && code <= 'f' {
		return int(code - 'a' + 10)
	}
	if code >= 'A' && code <= 'F' {
		return int(code - 'A' + 10)
	}
	return -1
}

func hexByte(text string, index int, doubled bool) int {
	high := hexDigit(text[index])
	low := high
	if !doubled {
		low = hexDigit(text[index+1])
	}
	if high < 0 || low < 0 {
		return -1
	}
	return high*16 + low
}
