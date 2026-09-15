#!/usr/bin/env bun
/** Prove a grammar-free editor loads multiple portable Wasm grammars from one pack. */
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { hostTarget } from "../packages/cli/src/targets.ts";
const root=resolve(import.meta.dir,".."), target=hostTarget();
async function run(args:string[],cwd=root,env:Record<string,string|undefined>={}) {
    const child=Bun.spawn(args,{cwd,env:{...Bun.env,CGO_ENABLED:"0",...env},stdout:"pipe",stderr:"pipe"});
    const [status,out,err]=await Promise.all([child.exited,new Response(child.stdout).text(),new Response(child.stderr).text()]);
    if(status!==0)throw new Error(`${args.join(" ")}: ${err}\n${out}`);
    return out;
}
const scratch=mkdtempSync(join(tmpdir(),"quickgui-language-pack-"));
try {
    for(const args of [["-p","quickgui-editor"],["-p","quickgui","--features","editor"]]) {
        const graph=await run(["cargo","tree",...args,"--edges","normal","--prefix","none"]);
        if(/^tree-sitter-(?!highlight\b|language\b)[a-z]/m.test(graph))throw new Error("Default editor contains grammar crates");
    }
    const metadata=JSON.parse(await run(["cargo","metadata","--locked","--format-version","1"]));
    const grammars:Record<string,string>={}, languages:unknown[]=[];
    for(const name of ["lua","rust"]) {
        const source=dirname(metadata.packages.find((p:{name:string})=>p.name===`tree-sitter-${name}`).manifest_path);
        grammars[name]=join(scratch,`${name}.wasm`);
        await run(["bunx","--package","tree-sitter-cli@0.26.7","tree-sitter","build","--wasm","--output",grammars[name]!,source]);
        languages.push({name,grammar:name,aliases:[`${name}-fence`],extensions:[name==="rust"?"rs":"lua"],queries:{highlights:join(source,"queries/highlights.scm")}});
    }
    const config=join(scratch,"config.json"), pack=join(scratch,"languages.qglang");
    writeFileSync(config,JSON.stringify({grammars,languages}));
    await run([process.execPath,"packages/cli/src/cli.ts","pack-languages",config,"--languages","lua,rust","--out",pack]);
    const library=(name:string)=>join(root,name==="host"?"packages/native/lib":"extensions/editor/lib",target,process.platform==="win32"?`quickgui_${name}.dll`:`libquickgui_${name}.${process.platform==="darwin"?"dylib":"so"}`);
    const env={QUICKGUI_TEST_CORE:library("host"),QUICKGUI_TEST_EDITOR:library("editor"),QUICKGUI_TEST_LANGUAGE_PACK:pack};
    console.log((await run(["go","test","./internal/ffi","-run","TestEditorLanguagePackSmoke","-count=1","-v"],join(root,"go"),env)).trim());
    console.log((await run(["cargo","test","-p","quickgui-editor","--test","languages","--","--test-threads=1"],root,env)).trim());
    console.log("[languages] One Lua + Rust Wasm pack: load, aliases, syntax colors, live refresh and errors passed");
} finally { rmSync(scratch,{recursive:true,force:true}); }
