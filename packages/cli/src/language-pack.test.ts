import { expect,test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { buildLanguagePack } from "./language-pack.ts";
import { parseCliArgs } from "./args.ts";

function member(bytes:Buffer,path:string):Buffer {
 for(let offset=0;offset+512<=bytes.length;) {
  const field=bytes.subarray(offset,offset+100), end=field.indexOf(0);
  const name=field.subarray(0,end<0?100:end).toString("utf8");
  if(!name)break;
  const size=parseInt(bytes.subarray(offset+124,offset+136).toString("ascii"),8);
  if(name===path)return bytes.subarray(offset+512,offset+512+size);
  offset+=512+Math.ceil(size/512)*512;
 }
 throw new Error("missing "+path);
}
test("packs only selected languages and dependencies, deduplicating grammars",()=>{
 const root=mkdtempSync(join(tmpdir(),"quickgui-pack-test-"));
 try {
  const wasm=Buffer.from([0,97,115,109,1,0,0,0]);
  writeFileSync(join(root,"parser.wasm"),wasm);
  writeFileSync(join(root,"highlights.scm"),"(identifier) @function");
  const config=join(root,"config.json"),output=join(root,"languages.qglang");
  writeFileSync(config,JSON.stringify({grammars:{shared:"parser.wasm",unused:"not-present.wasm"},languages:[{name:"host",grammar:"shared",dependencies:["nested"],queries:{highlights:"highlights.scm"}},{name:"nested",grammar:"shared",queries:{}},{name:"unused",grammar:"unused",queries:{}}]}));
  expect(buildLanguagePack(config,output,["host"])).toEqual(["host","nested"]);
  const bytes=readFileSync(output);
  const pack=JSON.parse(member(bytes,"manifest.json").toString("utf8"));
  expect(Object.keys(pack.grammars)).toEqual(["shared"]);
  expect(member(bytes,pack.grammars.shared)).toEqual(wasm);
  expect(pack.languages[0].queries.highlights).toBe("(identifier) @function");
  const before=readFileSync(output);
  expect(()=>buildLanguagePack(config,output,["missing"])).toThrow("no language");
  expect(readFileSync(output)).toEqual(before);
  writeFileSync(join(root,"parser.wasm"),"not wasm");
  expect(()=>buildLanguagePack(config,output,["host"])).toThrow("compiled Wasm");
  expect(readFileSync(output)).toEqual(before);
 } finally {rmSync(root,{recursive:true,force:true});}
});

test("pack CLI accepts an explicit selection and requires an output",()=>{
 expect(parseCliArgs(["pack-languages","config.json","--languages","lua,rust","--out","languages.qglang"])).toEqual({command:"pack-languages",config:"config.json",languages:["lua","rust"],output:"languages.qglang"});
 expect(()=>parseCliArgs(["pack-languages","config.json"])).toThrow("--out");
 expect(parseCliArgs(["pack-languages","--help"])).toEqual({command:"help",topic:"pack-languages"});
});

test("repository grammars must pin a commit and unselected sources are not fetched",()=>{
 const root=mkdtempSync(join(tmpdir(),"quickgui-pack-repository-"));
 try {
  writeFileSync(join(root,"parser.wasm"),Buffer.from([0,97,115,109,1,0,0,0]));
  const config=join(root,"config.json"),output=join(root,"languages.qglang");
  writeFileSync(config,JSON.stringify({grammars:{local:"parser.wasm",remote:{repository:"https://example.invalid/grammar",rev:"main"}},languages:[{name:"local",grammar:"local",queries:{}},{name:"remote",grammar:"remote"}]}));
  expect(buildLanguagePack(config,output,["local"])).toEqual(["local"]);
  expect(()=>buildLanguagePack(config,output,["remote"])).toThrow("pinned commit");
 } finally {rmSync(root,{recursive:true,force:true});}
});
