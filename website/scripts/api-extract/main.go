// Extract public API declarations with Go's parser so docs preserve real types.
package main

import (
	"bytes"
	"encoding/json"
	"go/ast"
	"go/parser"
	"go/printer"
	"go/token"
	"os"
	"path/filepath"
	"strings"
)

type Field struct {
	Name        string `json:"name"`
	Type        string `json:"type"`
	Description string `json:"description"`
	Source      string `json:"source"`
}
type Decl struct {
	Name        string  `json:"name"`
	Receiver    string  `json:"receiver,omitempty"`
	Signature   string  `json:"signature"`
	Description string  `json:"description"`
	Source      string  `json:"source"`
	Fields      []Field `json:"fields,omitempty"`
}

func main() {
	fset := token.NewFileSet()
	out := []Decl{}
	files, err := filepath.Glob("go/ui/*.go")
	if err != nil {
		panic(err)
	}
	files = append(files, "extensions/terminal/terminal.go", "extensions/editor/editor.go", "extensions/markdown/markdown.go")
	render := func(n any) string {
		var b bytes.Buffer
		if err := printer.Fprint(&b, fset, n); err != nil {
			panic(err)
		}
		return b.String()
	}
	for _, path := range files {
		if strings.HasSuffix(path, "_test.go") {
			continue
		}
		prefix := ""
		if strings.Contains(path, "extensions/terminal/") {
			prefix = "terminal."
		}
		if strings.Contains(path, "extensions/editor/") {
			prefix = "editor."
		}
		if strings.Contains(path, "extensions/markdown/") {
			prefix = "markdown."
		}
		file, err := parser.ParseFile(fset, path, nil, parser.ParseComments)
		if err != nil {
			panic(err)
		}
		source := func(p token.Pos) string { return path + "#L" + itoa(fset.Position(p).Line) }
		for _, d := range file.Decls {
			switch d := d.(type) {
			case *ast.FuncDecl:
				if !d.Name.IsExported() {
					continue
				}
				receiver := ""
				if d.Recv != nil {
					receiver = render(d.Recv.List[0].Type)
				}
				out = append(out, Decl{Name: prefix + d.Name.Name, Receiver: receiver, Signature: strings.Replace(render(d.Type), "func(", d.Name.Name+"(", 1), Description: d.Doc.Text(), Source: source(d.Pos())})
			case *ast.GenDecl:
				for _, spec := range d.Specs {
					if t, ok := spec.(*ast.TypeSpec); ok && t.Name.IsExported() {
						entry := Decl{Name: prefix + t.Name.Name, Signature: render(t.Type), Description: d.Doc.Text(), Source: source(t.Pos())}
						if st, ok := t.Type.(*ast.StructType); ok {
							for _, f := range st.Fields.List {
								desc := f.Doc.Text() + f.Comment.Text()
								typ := render(f.Type)
								if len(f.Names) == 0 {
									entry.Fields = append(entry.Fields, Field{Type: typ, Description: desc, Source: source(f.Pos())})
								}
								for _, n := range f.Names {
									if n.IsExported() {
										entry.Fields = append(entry.Fields, Field{Name: n.Name, Type: typ, Description: desc, Source: source(f.Pos())})
									}
								}
							}
						}
						out = append(out, entry)
					}
				}
			}
		}
	}
	if err := json.NewEncoder(os.Stdout).Encode(out); err != nil {
		panic(err)
	}
}
func itoa(n int) string {
	if n == 0 {
		return "0"
	}
	b := []byte{}
	for n > 0 {
		b = append([]byte{byte('0' + n%10)}, b...)
		n /= 10
	}
	return string(b)
}
