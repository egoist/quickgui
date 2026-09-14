package ui

import (
	"fmt"
	"math"
	"strconv"
	"strings"

	gui "github.com/egoist/quickgui/go/ui"
	"quickgui.example/quick-git/internal/git"
)

const (
	historyRowHeight = 26
	graphLaneWidth   = 14
	graphInset       = 6
)

type graphLayer struct {
	Color  int
	Source string
}

func graphLaneX(lane int) float64 {
	return graphInset + float64(lane)*graphLaneWidth + graphLaneWidth/2
}

// Each layer is an SVG mask tinted with one lane color. Paths reach the row's
// exact edges so the virtualized rows form a continuous graph.
func paintGraphRow(row git.GraphRow, width, height float64) []graphLayer {
	middle := height / 2
	radius := math.Min(graphLaneWidth/2, math.Max(1, middle-1))
	segments := map[int][]string{}
	var colors []int
	add := func(color int, path string) {
		if _, ok := segments[color]; !ok {
			colors = append(colors, color)
		}
		segments[color] = append(segments[color], path)
	}
	x := graphLaneX(row.Lane)
	for _, lane := range row.Passing {
		add(lane.Color, "M"+strconv.FormatFloat(graphLaneX(lane.Lane), 'g', -1, 64)+" 0V"+strconv.FormatFloat(height, 'g', -1, 64))
	}
	if row.Incoming {
		add(row.Color, "M"+strconv.FormatFloat(x, 'g', -1, 64)+" 0V"+strconv.FormatFloat(middle, 'g', -1, 64))
	}
	for _, join := range row.Joins {
		from := graphLaneX(join.FromLane)
		if from == x {
			add(join.Color, "M"+strconv.FormatFloat(x, 'g', -1, 64)+" 0V"+strconv.FormatFloat(middle, 'g', -1, 64))
			continue
		}
		direction, sweep := 1.0, 0
		if from > x {
			direction, sweep = -1, 1
		}
		add(join.Color, "M"+strconv.FormatFloat(from, 'g', -1, 64)+" 0V"+strconv.FormatFloat(middle-radius, 'g', -1, 64)+"A"+strconv.FormatFloat(radius, 'g', -1, 64)+" "+strconv.FormatFloat(radius, 'g', -1, 64)+" 0 0 "+strconv.Itoa(sweep)+" "+strconv.FormatFloat(from+direction*radius, 'g', -1, 64)+" "+strconv.FormatFloat(middle, 'g', -1, 64)+"H"+strconv.FormatFloat(x, 'g', -1, 64))
	}
	for _, edge := range row.Edges {
		to := graphLaneX(edge.ToLane)
		if to == x {
			add(edge.Color, "M"+strconv.FormatFloat(x, 'g', -1, 64)+" "+strconv.FormatFloat(middle, 'g', -1, 64)+"V"+strconv.FormatFloat(height, 'g', -1, 64))
			continue
		}
		direction, sweep := -1.0, 0
		if to > x {
			direction, sweep = 1, 1
		}
		add(edge.Color, "M"+strconv.FormatFloat(x, 'g', -1, 64)+" "+strconv.FormatFloat(middle, 'g', -1, 64)+"H"+strconv.FormatFloat(to-direction*radius, 'g', -1, 64)+"A"+strconv.FormatFloat(radius, 'g', -1, 64)+" "+strconv.FormatFloat(radius, 'g', -1, 64)+" 0 0 "+strconv.Itoa(sweep)+" "+strconv.FormatFloat(to, 'g', -1, 64)+" "+strconv.FormatFloat(middle+radius, 'g', -1, 64)+"V"+strconv.FormatFloat(height, 'g', -1, 64))
	}
	if _, ok := segments[row.Color]; !ok {
		add(row.Color, "")
	}
	layers := make([]graphLayer, 0, len(colors))
	for _, color := range colors {
		var body strings.Builder
		if path := strings.Join(segments[color], ""); path != "" {
			fmt.Fprintf(&body, `<path d="%s" fill="none" stroke="#000" stroke-width="2"/>`, path)
		}
		if color == row.Color {
			fmt.Fprintf(&body, `<circle cx="%g" cy="%g" r="4" fill="#000"/>`, x, middle)
		}
		layers = append(layers, graphLayer{Color: color, Source: "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"" + strconv.FormatFloat(width, 'g', -1, 64) + "\" height=\"" + strconv.FormatFloat(height, 'g', -1, 64) + "\" viewBox=\"0 0 " + strconv.FormatFloat(width, 'g', -1, 64) + " " + strconv.FormatFloat(height, 'g', -1, 64) + "\">" + body.String() + "</svg>"})
	}
	return layers
}

func historyGraph(row func() *git.GraphRow, width func() float64) *gui.Element {
	app := UseApp()
	return gui.View().
		Child(

			gui.KeyedFor(
				func() []graphLayer {
					if current := row(); current != nil {
						return paintGraphRow(*current, width(), historyRowHeight)
					}
					return nil
				},
				func(layer graphLayer) any { return layer.Color },
				func(layer func() graphLayer, _ func() int) *gui.Element {
					return gui.SVG().
						Position("absolute").
						Left(0).
						Top(0).
						Width(width()).
						Height(historyRowHeight).
						TextColor(func() string {
							palette := app.Theme().Graph
							return palette[layer().Color%len(palette)]
						}).
						Value(func() string { return layer().Source })

				},
				nil,
			),
		).
		Position("relative").
		Width(width()).
		Height(historyRowHeight).
		FlexShrink(0).
		Overflow("hidden")
}
