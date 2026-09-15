import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  renameSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { createHash } from "node:crypto";
import { dirname, join, resolve, sep } from "node:path";
import { CliError } from "./error.ts";
import { createTar, type TarEntry } from "./packaging/archive.ts";

const MAX_PACK = 64 * 1024 * 1024;
const MAX_WASM = 16 * 1024 * 1024;
const MAX_QUERIES = 1024 * 1024;
type QueryFiles = string | string[];
type QuerySources = { highlights?: QueryFiles; injections?: QueryFiles; locals?: QueryFiles };
interface GrammarRepository {
  repository: string;
  rev: string;
  path?: string;
  queries?: QuerySources;
}
interface SourceLanguage {
  name: string;
  grammar: string;
  aliases?: string[];
  extensions?: string[];
  filenames?: string[];
  dependencies?: string[];
  queries?: QuerySources;
}
interface PackSource {
  grammars: Record<string, string | GrammarRepository>;
  languages: SourceLanguage[];
  licenses?: Record<string, string>;
}

const TREE_SITTER_CLI = "tree-sitter-cli@0.26.7";
function run(args: string[], cwd: string): void {
  const result = Bun.spawnSync(args, {cwd,env:{...process.env,GIT_TERMINAL_PROMPT:"0"},stdout:"pipe",stderr:"pipe"});
  if (result.exitCode !== 0) throw new CliError(`${args[0]} failed: ${result.stderr.toString().trim()}`);
}

/** Resolve a compiled grammar or compile an immutable repository revision into the build cache. */
function resolveGrammar(source: string | GrammarRepository, root: string): {wasm: string; queries?: QuerySources; license?: string} {
  if (typeof source === "string") return {wasm:resolve(root,source)};
  if (!source || typeof source.repository !== "string" || !source.repository.startsWith("https://") || !/^[a-f0-9]{40}$/.test(source.rev)) {
    throw new CliError("Grammar repositories require an HTTPS repository and a full pinned commit rev");
  }
  if (source.path !== undefined && (typeof source.path !== "string" || source.path.startsWith("/") || source.path.includes("\\") || source.path.split("/").includes(".."))) throw new CliError("Invalid grammar repository path");
  const key=createHash("sha256").update(JSON.stringify([source.repository,source.rev,source.path??"",TREE_SITTER_CLI])).digest("hex");
  const cache=join(root,".quickgui","language-packs",key);
  mkdirSync(cache,{recursive:true});
  const checkout=join(cache,"source");
  if (!existsSync(checkout)) {
    const temporary=mkdtempSync(join(cache,"checkout-"));
    try {
      run(["git","init","--quiet",temporary],root);
      run(["git","-C",temporary,"remote","add","origin",source.repository],root);
      run(["git","-C",temporary,"fetch","--quiet","--depth=1","origin",source.rev],root);
      run(["git","-C",temporary,"checkout","--quiet","--detach","FETCH_HEAD"],root);
      renameSync(temporary,checkout);
    } finally {if(existsSync(temporary))rmSync(temporary,{recursive:true,force:true});}
  }
  const checkoutRoot=realpathSync(checkout);
  const member=(path: string): string => {
    const result=realpathSync(resolve(checkout,path));
    if(result!==checkoutRoot&&!result.startsWith(checkoutRoot+sep))throw new CliError("Grammar source path escapes its checkout");
    return result;
  };
  const wasm=join(cache,"grammar.wasm");
  if (!existsSync(wasm)) {
    console.log(`[languages] Compiling ${source.repository} at ${source.rev}`);
    const temporary=mkdtempSync(join(cache,"compile-"));
    try {
      const output=join(temporary,"grammar.wasm");
      run([process.execPath,"x","--package",TREE_SITTER_CLI,"tree-sitter","build","--wasm","--output",output,member(source.path??".")],root);
      renameSync(output,wasm);
    } finally {rmSync(temporary,{recursive:true,force:true});}
  }
  const queries:QuerySources={};
  for(const key of ["highlights","injections","locals"] as const) {
    const configured=source.queries?.[key];
    if(configured!==undefined) {
      const files=typeof configured==="string"?[configured]:configured;
      if(!Array.isArray(files)||files.length>32||files.some(file=>typeof file!=="string"))throw new CliError("Invalid repository query paths");
      queries[key]=files.map(member);
    } else if(existsSync(join(checkout,"queries",`${key}.scm`))) queries[key]=member(`queries/${key}.scm`);
  }
  const license=["LICENSE","LICENSE.md","LICENSE.txt"].find(name=>existsSync(join(checkout,name)));
  return {wasm,queries,...(license?{license:member(license)}:{})};
}

function boundedRead(path: string, limit: number): Buffer {
  if (!statSync(path).isFile() || statSync(path).size > limit)
    throw new CliError(`Language pack input exceeds its limit or is not a file: ${path}`);
  const bytes = readFileSync(path);
  if (bytes.length > limit) throw new CliError(`Language pack input exceeds its limit: ${path}`);
  return bytes;
}
function names(values: string[] | undefined): string[] {
  if (values === undefined) return [];
  if (
    !Array.isArray(values) ||
    values.length > 32 ||
    values.some(
      (value) => typeof value !== "string" || !value || value.length > 255 || /[\0/\\]/.test(value),
    )
  )
    throw new CliError("Invalid language aliases, extensions or filenames");
  return values;
}

/** Bundle raw Wasm grammars and a JSON manifest into one portable binary pack. */
export function buildLanguagePack(
  configPath: string,
  outputPath: string,
  selected?: string[],
): string[] {
  const config = resolve(configPath),
    root = dirname(config);
  const source = JSON.parse(boundedRead(config, MAX_QUERIES).toString("utf8")) as PackSource;
  if (
    !source.grammars ||
    !Array.isArray(source.languages) ||
    source.languages.length === 0 ||
    source.languages.length > 128
  )
    throw new CliError("Language pack config requires grammars and 1–128 languages");
  const available = new Map<string, SourceLanguage>();
  for (const language of source.languages) {
    if (
      !language ||
      typeof language.name !== "string" ||
      !/^[a-z0-9][a-z0-9_.+#-]{0,63}$/.test(language.name) ||
      available.has(language.name)
    )
      throw new CliError("Invalid or duplicate language name");
    available.set(language.name, language);
  }
  const chosen = new Set<string>();
  const select = (name: string) => {
    if (chosen.has(name)) return;
    const language = available.get(name);
    if (!language) throw new CliError(`Language pack config has no language '${name}'`);
    chosen.add(name);
    if (
      language.dependencies !== undefined &&
      (!Array.isArray(language.dependencies) || language.dependencies.length > 128)
    )
      throw new CliError("Invalid language dependencies");
    for (const dependency of language.dependencies ?? []) select(dependency);
  };
  for (const name of selected ?? available.keys()) select(name);
  if (chosen.size === 0) throw new CliError("Select at least one language");
  const query = (files: QueryFiles | undefined): string => {
    if (files === undefined) return "";
    const paths = typeof files === "string" ? [files] : files;
    if (
      !Array.isArray(paths) ||
      paths.length > 32 ||
      paths.some((path) => typeof path !== "string" || !path)
    )
      throw new CliError("Queries must be file paths");
    const result = paths
      .map((path) => boundedRead(resolve(root, path), MAX_QUERIES).toString("utf8"))
      .join("\n");
    if (Buffer.byteLength(result) > MAX_QUERIES)
      throw new CliError("Language queries exceed 1 MiB");
    return result;
  };
  const grammars: Record<string, string> = Object.create(null),
    licenses: Record<string, string> = Object.create(null);
  const modules: TarEntry[] = [];
  const resolved = new Map<string, ReturnType<typeof resolveGrammar>>();
  const languages = [...chosen].map((name) => {
    const language = available.get(name)!;
    if (typeof language.grammar !== "string" || !/^[A-Za-z0-9_]{1,64}$/.test(language.grammar))
      throw new CliError("Invalid Wasm grammar name");
    if (!grammars[language.grammar]) {
      const input = source.grammars[language.grammar];
      if (!input)
        throw new CliError(`Missing grammar '${language.grammar}'`);
      const grammar = resolveGrammar(input,root);
      resolved.set(language.grammar,grammar);
      const wasm = boundedRead(grammar.wasm, MAX_WASM);
      if (!wasm.subarray(0, 8).equals(Buffer.from([0, 97, 115, 109, 1, 0, 0, 0])))
        throw new CliError(`Grammar '${language.grammar}' must be compiled Wasm`);
      const member = `grammars/${language.grammar}.wasm`;
      grammars[language.grammar] = member;
      modules.push({ path: member, data: wasm });
      const license = source.licenses?.[language.grammar] ?? grammar.license;
      if (license)
        licenses[language.grammar] = boundedRead(resolve(root, license), MAX_QUERIES).toString(
          "utf8",
        );
    }
    const defaults = resolved.get(language.grammar)?.queries;
    const queries = {
      highlights: query(language.queries?.highlights ?? defaults?.highlights),
      injections: query(language.queries?.injections ?? defaults?.injections),
      locals: query(language.queries?.locals ?? defaults?.locals),
    };
    if (
      Object.values(queries).reduce((sum, value) => sum + Buffer.byteLength(value), 0) > MAX_QUERIES
    )
      throw new CliError("Language queries exceed 1 MiB");
    return {
      name,
      grammar: language.grammar,
      aliases: names(language.aliases),
      extensions: names(language.extensions),
      filenames: names(language.filenames),
      queries,
    };
  });
  const manifest = Buffer.from(
    JSON.stringify({ formatVersion: 1, grammars, languages, licenses }) + "\n",
  );
  if (manifest.length > 16 * 1024 * 1024)
    throw new CliError("Language pack manifest exceeds 16 MiB");
  const contents = createTar([{ path: "manifest.json", data: manifest }, ...modules]);
  if (contents.byteLength > MAX_PACK) throw new CliError("Language pack exceeds 64 MiB");
  const output = resolve(outputPath);
  if (existsSync(output) && statSync(output).isFile() && statSync(output).size === contents.byteLength && readFileSync(output).equals(contents)) return languages.map(language => language.name);
  mkdirSync(dirname(output), { recursive: true });
  const scratch = mkdtempSync(join(dirname(output), ".language-pack-"));
  try {
    const path = join(scratch, "pack");
    writeFileSync(path, contents);
    renameSync(path, output);
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
  return languages.map((language) => language.name);
}
