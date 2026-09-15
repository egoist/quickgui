import { expect, test } from "bun:test";
import { createSignal, flush } from "solid-js";
import { NativeNodeTag, PropertyCode, QuickGuiEvent, type NativeNode } from "@quickgui/native";
import { createComponent } from "@quickgui/solid";
import { CodeBlock, DiffView, Editor, loadLanguagePack } from "@quickgui/extension-editor";
import { invokeReplies, lastCall } from "../../../../packages/native/test/fake-binding.ts";

const props = (node: NativeNode) => JSON.parse(node.properties.get(PropertyCode.ExtensionProps) as string);

test("language packs use the editor package service and propagate native errors", async () => {
 const method="extension/editor/load-language-pack";
 try {
  invokeReplies.set(method,["lua","rust"]);
  expect(await loadLanguagePack("/resources/lua/language.json")).toEqual(["lua","rust"]);
  expect(lastCall("invoke").args).toEqual([method,{path:"/resources/lua/language.json"}]);
  invokeReplies.set(method,new Error("unsupported Tree-sitter grammar ABI"));
  await expect(loadLanguagePack("/resources/lua/language.json")).rejects.toThrow("unsupported Tree-sitter grammar ABI");
  invokeReplies.set(method,null);
  await expect(loadLanguagePack("/resources/lua/language.json")).rejects.toThrow("Invalid language registration reply");
  await expect(loadLanguagePack("")).rejects.toThrow("pack path");
 } finally { invokeReplies.delete(method); }
});

test("editor package keeps QuickGUI runtimes as peers", async () => {
 const manifest = await Bun.file(new URL("../../package.json", import.meta.url)).json();
 expect(manifest.dependencies).toBeUndefined();
 expect(manifest.peerDependencies).toMatchObject({"@quickgui/native":"workspace:*","@quickgui/solid":"workspace:*","solid-js":"2.0.0-rc.8"});
 expect(manifest.exports["."]).toBe("./js/index.ts");
});
test("editor updates properties and routes input through the generic component boundary", () => {
 const [source,setSource] = createSignal("fn main() {}");
 const [language,setLanguage] = createSignal("rust");
 const editor = createComponent(Editor, {
  get value(){return source();}, get language(){return language();}, tabSize:2,
  gutterColor:"#8892a0",syntaxTheme:{keyword:"#ff66cc"},style:{height:320},
  onInput(event){setSource(event.value ?? "");},
 });
 expect(editor.tag).toBe(NativeNodeTag.Extension);
 expect(editor.properties.get(PropertyCode.ExtensionPackage)).toBe("editor");
 expect(editor.properties.get(PropertyCode.ExtensionComponent)).toBe("editor");
 expect(props(editor)).toMatchObject({value:"fn main() {}",language:"rust",tabSize:2,syntaxTheme:{keyword:"#ff66cc"}});
 setLanguage("go");flush();
 expect(props(editor).language).toBe("go");
 editor.listeners.get("componentchange")!(new QuickGuiEvent("componentchange",editor,JSON.stringify({kind:"input",value:"package main"})));
 expect(props(editor).value).toBe("package main");
 expect(editor.properties.get(PropertyCode.Height)).toBe(320);
});
test("diff and code-block properties remain extension-owned", () => {
 const diff = DiffView({oldText:"one\n",newText:"two\n",options:{layout:"unified",indicators:"classic",backgrounds:false,wrap:true},theme:{addedColor:"#44cc66"}});
 expect(diff.tag).toBe(NativeNodeTag.Extension);
 expect(diff.properties.get(PropertyCode.ExtensionComponent)).toBe("diff-view");
 expect(props(diff)).toMatchObject({oldText:"one\n",newText:"two\n",options:{layout:"unified",wrap:true},theme:{addedColor:"#44cc66"}});
 const block=CodeBlock({value:"const answer = 42;",language:"typescript",lineNumbers:true,wrap:false});
 expect(block.properties.get(PropertyCode.ExtensionComponent)).toBe("code-block");
 expect(props(block)).toMatchObject({value:"const answer = 42;",lineNumbers:true,wrap:false});
});

test("appearance is supplied by the caller, not by the binding", () => {
 const bare = props(CodeBlock({value:"code"}));
 expect(bare.style).toBeUndefined();
 expect(bare.syntaxTheme).toBeUndefined();
 expect(bare.contentPadding).toBeUndefined();
 const [left,setLeft]=createSignal(12);
 const block=createComponent(CodeBlock,{value:"code",get contentPadding(){return {left:left(),right:12,vertical:8};},style:{borderWidth:1,borderRadius:6}});
 expect(props(block)).toMatchObject({contentPadding:{left:12,right:12,vertical:8},style:{borderWidth:1,borderRadius:6}});
 setLeft(0);flush();
 expect(props(block).contentPadding.left).toBe(0);
 const diff=DiffView({oldText:"a",newText:"b",presentation:{headerPadding:12,gutterPadding:8},theme:{mutedColor:"#888"}});
 expect(props(diff)).toMatchObject({presentation:{headerPadding:12,gutterPadding:8},theme:{mutedColor:"#888"}});
});
