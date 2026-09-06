import { unlinkSync, watch, type FSWatcher } from "node:fs";
import { tmpdir } from "node:os";
import { basename, isAbsolute, join, relative, resolve, sep } from "node:path";

import { buildProject, type BuildResult } from "./build.ts";
import { loadConfig, type ResolvedQuickGuiConfig } from "./config.ts";
import { CliError, errorMessage } from "./error.ts";
import { hostTarget, type QuickGuiTarget } from "./targets.ts";

export interface DevOptions {
  project: string;
  configFile: string;
  once: boolean;
  launch: boolean;
  target?: QuickGuiTarget;
  signingIdentity?: string;
}

type AppProcess = Bun.Subprocess<"ignore", "inherit", "inherit">;

let nextReadySocket = 1;

interface ExitingProcess {
  readonly exited: Promise<number>;
}

/** @internal */
export class ActiveProcessMonitor<T extends ExitingProcess> {
  #active: T | undefined;
  #closed = false;
  #onExit: (status: number) => void;

  constructor(onExit: (status: number) => void) {
    this.#onExit = onExit;
  }

  get active(): T | undefined {
    return this.#active;
  }

  activate(child: T): void {
    this.#active = child;
    void child.exited.then((status) => {
      if (!this.#closed && this.#active === child) this.#onExit(status);
    });
  }

  close(): void {
    this.#closed = true;
  }
}

export async function runDev(options: DevOptions): Promise<number> {
  const projectRoot = resolve(options.project);
  const target = options.target ?? hostTarget();
  const host = hostTarget();
  if (target !== host) {
    throw new CliError(
      `Development apps must run on the host target (${host}); build ${target} on a matching host`,
    );
  }

  let config = await loadConfig(projectRoot, options.configFile);
  let build = await packageDevelopmentHost(config, target, options.signingIdentity);
  console.log(`[quickgui] Development app: ${build.artifactPath}`);
  if (!options.launch) return 0;

  if (options.once) {
    const child = await launchApplication(build, config);
    console.log(`[quickgui] App ready (pid ${child.pid})`);
    return await child.exited;
  }

  const abortController = new AbortController();
  let stopping = false;
  const processes = new ActiveProcessMonitor<AppProcess>((status) => {
    if (stopping || abortController.signal.aborted) return;
    console.log(`[quickgui] App exited (status ${status}); stopping watcher`);
    abortController.abort();
  });
  let reloadQueued = false;
  let reloadPromise: Promise<void> | undefined;
  let changedPath: string | undefined;
  let debounce: ReturnType<typeof setTimeout> | undefined;

  const reload = async (): Promise<void> => {
    try {
      const nextConfig = await loadConfig(projectRoot, options.configFile);
      const nextBuild = await packageDevelopmentHost(
        nextConfig,
        target,
        options.signingIdentity,
      );
      const candidate = await launchApplication(nextBuild, nextConfig, abortController.signal);
      if (stopping) {
        await stopApplication(candidate);
        return;
      }
      const previous = processes.active;
      processes.activate(candidate);
      config = nextConfig;
      build = nextBuild;
      if (previous) await stopApplication(previous);
      console.log(`[quickgui] Reloaded (pid ${candidate.pid})`);
    } catch (error) {
      if (!stopping) {
        console.error(
          `[quickgui] Reload failed; keeping the previous app running.\n${errorMessage(error)}`,
        );
      }
    }
  };

  const queueReload = (path: string | undefined): void => {
    if (stopping) return;
    changedPath = path;
    reloadQueued = true;
    if (reloadPromise) return;
    reloadPromise = (async () => {
      while (reloadQueued && !stopping) {
        reloadQueued = false;
        await reload();
      }
    })().finally(() => {
      reloadPromise = undefined;
      if (reloadQueued && !stopping) queueReload(changedPath);
    });
  };

  let watcher: FSWatcher;
  try {
    watcher = watch(projectRoot, { recursive: true }, (_event, filename) => {
      const path = filename ? resolve(projectRoot, String(filename)) : undefined;
      if (path && shouldIgnoreChange(projectRoot, path, config.outDir, config.modules.directory)) {
        return;
      }
      if (debounce) clearTimeout(debounce);
      debounce = setTimeout(() => queueReload(path), 80);
    });
  } catch (error) {
    throw new CliError(`Could not watch ${projectRoot}`, { cause: error });
  }
  watcher.on("error", (error) => {
    console.error(`[quickgui] File watcher error: ${errorMessage(error)}`);
  });

  try {
    try {
      const child = await launchApplication(build, config, abortController.signal);
      processes.activate(child);
      console.log(`[quickgui] App ready (pid ${child.pid}); watching for changes`);
    } catch (error) {
      console.error(`[quickgui] App failed to start; watching for changes.\n${errorMessage(error)}`);
    }

    await waitForShutdown(abortController);
  } finally {
    stopping = true;
    processes.close();
    abortController.abort();
    if (debounce) clearTimeout(debounce);
    watcher.close();
    if (reloadPromise) await reloadPromise;
    if (processes.active) await stopApplication(processes.active);
  }
  return 0;
}

async function packageDevelopmentHost(
  config: ResolvedQuickGuiConfig,
  target: QuickGuiTarget,
  signingIdentity?: string,
): Promise<BuildResult> {
  const started = performance.now();
  const result = await buildProject(config, {
    mode: "development",
    target,
    ...(signingIdentity ? { signingIdentity } : {}),
  });
  console.log(`[quickgui] Packaged dev host in ${Math.round(performance.now() - started)} ms`);
  return result;
}

async function launchApplication(
  build: BuildResult,
  config: ResolvedQuickGuiConfig,
  signal?: AbortSignal,
): Promise<AppProcess> {
  let ready = false;
  let resolveReady!: () => void;
  let rejectReady!: (error: Error) => void;
  const readyPromise = new Promise<void>((resolvePromise, rejectPromise) => {
    resolveReady = resolvePromise;
    rejectReady = rejectPromise;
  });
  // The native host connects to this socket after its first ready event-loop turn.
  const socketPath = join(tmpdir(), `quickgui-ready-${process.pid}-${nextReadySocket++}.sock`);
  const server = Bun.listen({
    unix: socketPath,
    socket: {
      data(socket) {
        if (!ready) {
          ready = true;
          resolveReady();
        }
        socket.end();
      },
      open() {},
      error() {},
    },
  });
  const child = Bun.spawn([build.executablePath], {
    cwd: config.projectRoot,
    env: {
      ...process.env,
      NODE_ENV: "development",
      QUICKGUI_DEV: "1",
      QUICKGUI_READY_SOCKET: socketPath,
    },
    stdin: "ignore",
    stdout: "inherit",
    stderr: "inherit",
  });
  void child.exited.then((status) => {
    if (!ready) {
      rejectReady(
        new CliError(`Application exited before its first window was ready (status ${status})`),
      );
    }
  });

  const abort = (): void => rejectReady(new CliError("Development launch cancelled"));
  signal?.addEventListener("abort", abort, { once: true });
  const timeout = setTimeout(() => {
    rejectReady(new CliError("Application did not report a ready window within 15 seconds"));
  }, 15_000);
  try {
    await readyPromise;
    return child;
  } catch (error) {
    await stopApplication(child);
    throw error;
  } finally {
    clearTimeout(timeout);
    signal?.removeEventListener("abort", abort);
    server.stop(true);
    try {
      unlinkSync(socketPath);
    } catch {
      // The socket file was already removed.
    }
  }
}

async function stopApplication(child: AppProcess): Promise<void> {
  if (child.exitCode !== null || child.signalCode !== null) {
    await child.exited;
    return;
  }
  child.kill("SIGTERM");
  await Promise.race([child.exited, Bun.sleep(1_500)]);
  if (child.exitCode === null && child.signalCode === null) child.kill("SIGKILL");
  await child.exited;
}

/**
 * @internal
 * `modulesDir` is the native modules directory; the `index.ts` the CLI generates in each module
 * is ignored so writing it during a reload does not queue another one.
 */
export function shouldIgnoreChange(
  root: string,
  path: string,
  outDir: string,
  modulesDir?: string,
): boolean {
  const pathFromRoot = relative(root, path);
  if (pathFromRoot.startsWith("..") || isAbsolute(pathFromRoot)) return true;
  if (basename(path).endsWith(".bun-build")) return true;
  const parts = pathFromRoot.split(sep);
  if (
    parts.some((part) =>
      [".git", ".quickgui", ".zig-cache", "node_modules", "target", "vendor"].includes(part),
    )
  ) {
    return true;
  }
  if (modulesDir !== undefined) {
    const pathFromModules = relative(modulesDir, path);
    const moduleParts = pathFromModules.split(sep);
    if (
      !pathFromModules.startsWith("..") &&
      !isAbsolute(pathFromModules) &&
      moduleParts.length === 2 &&
      moduleParts[1] === "index.ts"
    ) {
      return true;
    }
  }
  const outDirFromRoot = relative(root, outDir);
  const outDirIsInsideRoot =
    outDirFromRoot !== "" && !outDirFromRoot.startsWith("..") && !isAbsolute(outDirFromRoot);
  if (!outDirIsInsideRoot) return false;
  const pathFromOutDir = relative(outDir, path);
  return pathFromOutDir === "" ||
    (!pathFromOutDir.startsWith("..") && !isAbsolute(pathFromOutDir));
}

async function waitForShutdown(abortController: AbortController): Promise<void> {
  if (abortController.signal.aborted) return;
  await new Promise<void>((resolvePromise) => {
    const requestShutdown = (): void => abortController.abort();
    // Bun's process typings narrow `removeListener` to their own event names; the emitter view
    // accepts the signal names that `once` registered.
    const signals: NodeJS.EventEmitter = process;
    const finish = (): void => {
      signals.removeListener("SIGINT", requestShutdown);
      signals.removeListener("SIGTERM", requestShutdown);
      resolvePromise();
    };
    process.once("SIGINT", requestShutdown);
    process.once("SIGTERM", requestShutdown);
    abortController.signal.addEventListener("abort", finish, { once: true });
  });
}
