#!/usr/bin/env bun
import { resolve } from "node:path";
import { restoreNativeLibArtifacts } from "./native-lib-artifacts.ts";

const root = resolve(process.argv[2] ?? resolve(import.meta.dir, ".."));
const restored = restoreNativeLibArtifacts(root);
if (restored.length > 0) console.log(`restored packages/${restored.join(", packages/")}`);
