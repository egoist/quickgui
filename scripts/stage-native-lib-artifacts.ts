#!/usr/bin/env bun
import { resolve } from "node:path";
import { stageNativeLibArtifacts } from "./native-lib-artifacts.ts";

const root = resolve(import.meta.dir, "..");
const destination = resolve(root, process.argv[2] ?? "target/native-libs");
stageNativeLibArtifacts(root, destination);
console.log(`staged native libraries at ${destination}`);
