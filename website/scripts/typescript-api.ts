/** Extract public JSX prop signatures from the actual TypeScript adapters. */
import { parse } from "@babel/parser";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import type { ComponentDoc } from "../src/lib/component-docs";
import type { ApiEntry, ApiSection } from "../src/lib/component-api";

const root = resolve(import.meta.dir, "../..");
type Declaration = { node: any; source: string; file: string };
const interfaces = new Map<string, Declaration>();
const aliases = new Map<string, Declaration>();
const functions = new Map<string, Declaration>();
const components = new Map<string, { root: string; parts: [string, string][] }>();
const compact = (text: string) => text.replace(/\s+/g, " ").trim();
const name = (node: any): string =>
  node?.type === "TSQualifiedName"
    ? `${name(node.left)}.${name(node.right)}`
    : (node?.name ?? node?.value ?? "");
for (const file of [
  "packages/solid/src/index.ts",
  "packages/solid/src/style-helpers.generated.ts",
  "packages/solid/src/swift-ui.ts",
  "packages/solid/src/router.ts",
  "extensions/editor/js/index.ts",
  "extensions/markdown/js/index.ts",
  "extensions/terminal/js/index.ts",
]) {
  const source = readFileSync(resolve(root, file), "utf8");
  const ast = parse(source, { sourceType: "module", plugins: ["typescript"] });
  const prefix = file.endsWith("swift-ui.ts") ? "swift-ui/" : "ui/";
  function visit(node: any, namespace = "") {
    if (!node) return;
    if (node.type === "ExportNamedDeclaration") return visit(node.declaration, namespace);
    if (node.type === "TSModuleDeclaration")
      return node.body.body.forEach((item: any) => visit(item, `${name(node.id)}.`));
    if (node.type === "TSInterfaceDeclaration")
      interfaces.set(prefix + namespace + name(node.id), { node, source, file });
    if (node.type === "TSTypeAliasDeclaration")
      aliases.set(prefix + namespace + name(node.id), {
        node: node.typeAnnotation,
        source,
        file,
      });
    if (node.type === "FunctionDeclaration" && node.id)
      functions.set(prefix + node.id.name, { node, source, file });
    if (node.type === "VariableDeclaration")
      for (const item of node.declarations) {
        if (
          item.init?.type === "CallExpression" &&
          item.init.callee?.object?.name === "Object" &&
          item.init.callee.property?.name === "assign"
        ) {
          const [base, parts] = item.init.arguments;
          components.set(prefix + name(item.id), {
            root: name(base),
            parts: (parts?.properties ?? [])
              .filter((p: any) => p.type === "ObjectProperty")
              .map((p: any) => [name(p.key), name(p.value)]),
          });
        }
      }
  }
  ast.program.body.forEach((item) => visit(item));
}
function keys(node: any, prefix: string): string[] {
  if (node?.type === "TSLiteralType") return [String(node.literal.value)];
  if (node?.type === "TSUnionType") return node.types.flatMap((part: any) => keys(part, prefix));
  if (node?.type === "TSTypeReference")
    return keys(
      (
        aliases.get(prefix + "JSX." + name(node.typeName)) ??
        aliases.get(prefix + name(node.typeName))
      )?.node,
      prefix,
    );
  return [];
}

function fieldEntry(field: any, source: string, file: string): ApiEntry | undefined {
  if (field.type !== "TSPropertySignature" && field.type !== "TSMethodSignature") return;
  const fieldName = name(field.key);
  if (!fieldName) return;
  const annotation = field.typeAnnotation?.typeAnnotation;
  const doc = (field.leadingComments ?? [])
    .map((comment: any) => comment.value.replace(/^\s*\* ?/gm, ""))
    .join("\n")
    .trim();
  return {
    name: fieldName,
    type: annotation
      ? compact(source.slice(annotation.start, annotation.end))
      : compact(source.slice(field.start, field.end)),
    description:
      compact(doc) ||
      (fieldName === "style"
        ? "Layout, typography, paint, and interaction-state styles. Arrays merge left to right."
        : `Declares ${fieldName}${field.optional ? " (optional)" : ""}. Reactive JSX expressions update this property in place.`),
    source: `${file}#L${field.loc.start.line}`,
  };
}

function insertEntry(entries: Map<string, ApiEntry>, entry: ApiEntry) {
  const previous = entries.get(entry.name);
  if (!previous || previous.type === "never") {
    entries.set(entry.name, entry);
  } else if (entry.type !== "never" && previous.type !== entry.type) {
    entries.set(entry.name, { ...previous, type: `${previous.type} | ${entry.type}` });
  }
}

function propertiesFromType(
  node: any,
  prefix: string,
  includeStyle: boolean,
  seen: Set<string>,
  declaration: Declaration,
): ApiEntry[] {
  if (node?.type === "TSParenthesizedType") {
    return propertiesFromType(node.typeAnnotation, prefix, includeStyle, seen, declaration);
  }
  if (node?.type === "TSIntersectionType" || node?.type === "TSUnionType") {
    const entries = new Map<string, ApiEntry>();
    for (const part of node.types) {
      for (const entry of propertiesFromType(
        part,
        prefix,
        includeStyle,
        new Set(seen),
        declaration,
      )) {
        insertEntry(entries, entry);
      }
    }
    return [...entries.values()];
  }
  if (node?.type === "TSTypeReference") {
    const typeName = name(node.typeName);
    const args = node.typeParameters?.params ?? [];
    const reference =
      typeName === "Omit" || typeName === "Partial" ? name(args[0]?.typeName ?? args[0]) : typeName;
    const excluded = new Set(typeName === "Omit" ? keys(args[1], prefix) : []);
    return properties(reference, prefix, includeStyle, new Set(seen)).filter(
      (entry) => !excluded.has(entry.name),
    );
  }
  if (node?.type === "TSTypeLiteral") {
    return node.members
      .map((field: any) => fieldEntry(field, declaration.source, declaration.file))
      .filter((entry: ApiEntry | undefined): entry is ApiEntry => Boolean(entry));
  }
  return [];
}

function properties(
  type: string,
  prefix: string,
  includeStyle = false,
  seen = new Set<string>(),
): ApiEntry[] {
  const qualified = prefix + type;
  if (seen.has(qualified) || (!includeStyle && (type === "Style" || type === "JSX.Style")))
    return [];
  seen.add(qualified);
  const declaration =
    interfaces.get(qualified) ??
    interfaces.get(prefix + "JSX." + type) ??
    interfaces.get("ui/" + type);
  if (!declaration) {
    const alias =
      aliases.get(qualified) ?? aliases.get(prefix + "JSX." + type) ?? aliases.get("ui/" + type);
    return alias ? propertiesFromType(alias.node, prefix, includeStyle, seen, alias) : [];
  }
  const { node, source, file } = declaration;
  const entries = new Map<string, ApiEntry>();
  for (const base of node.extends ?? []) {
    const baseName = name(base.expression),
      args = base.typeParameters?.params ?? [];
    const reference =
      baseName === "Omit" || baseName === "Partial" ? name(args[0]?.typeName) : baseName;
    const excluded = new Set(baseName === "Omit" ? keys(args[1], prefix) : []);
    for (const entry of properties(reference, prefix, includeStyle, new Set(seen)))
      if (!excluded.has(entry.name)) entries.set(entry.name, entry);
  }
  for (const field of node.body.body) {
    const entry = fieldEntry(field, source, file);
    if (entry) insertEntry(entries, entry);
  }
  return [...entries.values()];
}
export function typescriptApi(component: ComponentDoc): ApiSection[] {
  const prefix = component.kind + "/";
  const publicName = component.kind === "ui" && component.slug === "svg" ? "Svg" : component.name;
  const definition = components.get(prefix + publicName);
  const parts = definition
    ? [
        [publicName, definition.root],
        ...definition.parts.map(([part, fn]) => [`${publicName}.${part}`, fn]),
      ]
    : [[publicName, publicName]];
  return parts.flatMap(([label, fn]) => {
    const declaration = functions.get(prefix + fn);
    if (!declaration) return [];
    const { node, source, file } = declaration;
    const parameter = node.params[0];
    const reference = parameter?.typeAnnotation?.typeAnnotation;
    if (!reference) return [];
    const typeName = name(reference.typeName);
    return [
      {
        name: label!,
        signature: `${label}(props: ${compact(source.slice(reference.start, reference.end))}): ${node.returnType ? compact(source.slice(node.returnType.typeAnnotation.start, node.returnType.typeAnnotation.end)) : "NativeNode"}`,
        description: compact(
          (node.leadingComments ?? [])
            .map((comment: any) => comment.value.replace(/^\s*\* ?/gm, ""))
            .join("\n"),
        ),
        source: `${file}#L${node.loc.start.line}`,
        entries: properties(typeName, prefix, component.slug === "view"),
      },
    ];
  });
}
