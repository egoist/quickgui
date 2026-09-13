import { PROTOCOL_VERSION } from "../src/protocol.ts";
import type {
  NativeRouteDefinition,
  NativeRouteLocation,
  NativeRouterState,
  NativeRouteValue,
} from "../src/binding-types.ts";

/**
 * One shared fake native binding for the JavaScript host tests.
 *
 * `bun test` evaluates every test file in a single module registry, so `../src/index.ts` and its
 * `app` singleton are created once against whichever `mock.module("../src/binding.ts", …)` ran first.
 * Both host test files therefore install this one recorder and read their assertions from it.
 *
 * This module is test support only: it is deliberately absent from the package's published
 * `files` list.
 */
export type Call = { name: string; args: unknown[] };

export const calls: Call[] = [];

/** Native events waiting for the next `app.dispatchEvents()`. */
const pendingEvents: Record<string, unknown>[] = [];
const extensionListeners = new Map<number, (value: string) => void>();

export function emitExtensionEvent(session: number, value: unknown): void {
  extensionListeners.get(session)?.(JSON.stringify(value));
}

let clipboardItem: { entries: Record<string, unknown>[] } | null = null;
let findClipboardItem: { entries: Record<string, unknown>[] } | null = null;

function record(name: string, args: unknown[]): void {
  calls.push({ name, args });
}

export function callsNamed(name: string): Call[] {
  return calls.filter((call) => call.name === name);
}

export function lastCall(name: string): Call {
  const call = callsNamed(name).at(-1);
  if (!call) throw new Error(`no recorded ${name} call`);
  return call;
}

/** Queue native events for the next host dispatch. */
export function queueEvents(...events: Record<string, unknown>[]): void {
  pendingEvents.push(...events);
}

export const fakeBinding: Record<string, unknown> = {
  startExtension: (name: string, options: unknown, changed: (value: string) => void) => {
    const session = 1000 + callsNamed("startExtension").length;
    record("startExtension", [name, options, session]);
    extensionListeners.set(session, changed);
    return { session, ready: Promise.resolve() };
  },
  invoke: async (method: string, value: unknown) => {
    record("invoke", [method, value]);
    return null;
  },
  stopExtension: async (name: string, session: number) => {
    record("stopExtension", [name, session]);
    extensionListeners.delete(session);
  },
  protocolVersion: () => PROTOCOL_VERSION,
  createApp: (...args: unknown[]) => {
    record("createApp", args);
    return 1;
  },
  prepareApp: (...args: unknown[]) => record("prepareApp", args),
  isAppReady: () => true,
  destroyApp: (...args: unknown[]) => {
    record("destroyApp", args);
    return true;
  },
  configureApp: (...args: unknown[]) => record("configureApp", args),
  createWindow: (...args: unknown[]) => {
    record("createWindow", args);
    return 10 + callsNamed("createWindow").length;
  },
  createSystemPopover: (...args: unknown[]) => {
    record("createSystemPopover", args);
    return 100 + callsNamed("createSystemPopover").length;
  },
  createEmbeddedView: (...args: unknown[]) => {
    record("createEmbeddedView", args);
    return 200 + callsNamed("createEmbeddedView").length;
  },
  applyBatch: (...args: unknown[]) => {
    record("applyBatch", args);
    return 0;
  },
  focusNode: () => true,
  closeWindow: (...args: unknown[]) => {
    record("closeWindow", args);
    return true;
  },
  takeEvents: () => pendingEvents.splice(0),
  pumpApp: () => -1,
  performWindowAction: (...args: unknown[]) => record("performWindowAction", args),
  performWindowImageAction: (...args: unknown[]) => record("performWindowImageAction", args),
  performAppService: (...args: unknown[]) => record("performAppService", args),
  performAppMutation: (...args: unknown[]) => record("performAppMutation", args),
  showWindowPopupMenu: (...args: unknown[]) => record("showWindowPopupMenu", args),
  setApplicationMenu: (...args: unknown[]) => record("setApplicationMenu", args),
  setQuitInterception: (...args: unknown[]) => record("setQuitInterception", args),
  requestAppQuit: (...args: unknown[]) => {
    record("requestAppQuit", args);
    return true;
  },
  exitApp: (...args: unknown[]) => {
    record("exitApp", args);
    return true;
  },
  exitAppWithCode: (...args: unknown[]) => {
    record("exitAppWithCode", args);
    return true;
  },
  isApplicationPackaged: () => {
    record("isApplicationPackaged", []);
    return true;
  },
  getApplicationsFolderSupport: (...args: unknown[]) => {
    record("getApplicationsFolderSupport", args);
    return { supported: true, alreadyInstalled: false };
  },
  getWindowRestoreState: (...args: unknown[]) => {
    record("getWindowRestoreState", args);
    return {
      x: 32,
      y: 64,
      width: 900,
      height: 600,
      maximized: false,
      fullscreen: true,
      displayId: "3",
      displayUuid: "00112233-4455-6677-8899-aabbccddeeff",
      scaleFactor: 2,
    };
  },
  releaseSingleInstanceLock: () => true,
  readClipboard: () => clipboardItem,
  writeClipboard: (_app: number, item: { entries: Record<string, unknown>[] }) => {
    record("writeClipboard", [item]);
    clipboardItem = item;
  },
  readFindClipboard: (...args: unknown[]) => {
    record("readFindClipboard", args);
    return findClipboardItem;
  },
  writeFindClipboard: (_app: number, item: { entries: Record<string, unknown>[] }) => {
    record("writeFindClipboard", [item]);
    findClipboardItem = item;
  },
};

// The FFI facade only exposes the asynchronous host route. Keep one recorder so
// behavioral assertions still inspect the same native operations.
for (const [name, value] of Object.entries(fakeBinding)) {
  if (typeof value !== "function") continue;
  const hosted = name.replace(
    /^(create|prepare|configure|destroy|apply|close|focus|get|read|write|set|show|perform|request|release|relaunch|exit|add|clear|remove|dismiss)/,
    "$1Hosted",
  );
  if (hosted !== name && !(hosted in fakeBinding))
    fakeBinding[hosted] = (...args: unknown[]) => {
      const result = value(...args);
      return /^(createHosted|applyHosted|focusHosted|closeHosted|destroyHosted|performHostedWindow)/.test(
        hosted,
      )
        ? result
        : Promise.resolve(result);
    };
}
fakeBinding.requestHostedAppQuit = fakeBinding.requestAppQuit;
fakeBinding.windowReady = () => Promise.resolve();
fakeBinding.waitForHostedEvents = () => new Promise(() => {});
let dispatchEvents: () => void = () => {};
export function setEventDispatcher(dispatch: () => void) {
  dispatchEvents = dispatch;
}
const parents = new Map<number, number>();
for (const name of ["createHostedSystemPopover", "createHostedEmbeddedView"]) {
  const create = fakeBinding[name] as (...args: unknown[]) => number;
  fakeBinding[name] = (...args: unknown[]) => {
    const id = create(...args);
    parents.set(id, args[1] as number);
    return id;
  };
}
fakeBinding.closeHostedWindow = (app: number, id: number) => {
  (fakeBinding.closeWindow as (...args: unknown[]) => unknown)(app, id);
  const close = (window: number) => {
    for (const [child, parent] of parents)
      if (parent === window) {
        parents.delete(child);
        close(child);
      }
    queueEvents({ kind: "close", window, target: 0 });
  };
  close(id);
  queueMicrotask(() => dispatchEvents());
};

type PatternSegment =
  | { kind: "static"; value: string }
  | { kind: "param"; name: string; optional: boolean }
  | { kind: "wildcard"; name: string };

interface RouteSpecificity {
  staticSegments: number;
  requiredSegments: number;
  optionalSegments: number;
  wildcard: boolean;
  ancestryDepth: number;
}

interface CompiledRoute {
  id: string;
  routeIds: string[];
  segments: PatternSegment[];
  participates: boolean;
  specificity: RouteSpecificity;
}

function normalizePathname(path: string): string {
  const segments: string[] = [];
  for (const segment of path.split("/")) {
    if (segment === "" || segment === ".") continue;
    if (segment === "..") segments.pop();
    else segments.push(segment);
  }
  return `/${segments.join("/")}`;
}

function decodeComponent(component: string, plusAsSpace: boolean): string {
  const source = plusAsSpace ? component.replaceAll("+", " ") : component;
  try {
    return decodeURIComponent(source);
  } catch {
    return component;
  }
}

function parseQuery(search: string): NativeRouteValue[] {
  if (!search) return [];
  return search.split("&").map((pair) => {
    const separator = pair.indexOf("=");
    const name = separator === -1 ? pair : pair.slice(0, separator);
    const value = separator === -1 ? "" : pair.slice(separator + 1);
    return {
      name: decodeComponent(name, true),
      value: decodeComponent(value, true),
    };
  });
}

function parseLocation(destination: string, current?: NativeRouteLocation): NativeRouteLocation {
  const fallback: NativeRouteLocation = current ?? {
    href: "/",
    pathname: "/",
    search: "",
    hash: "",
    query: [],
  };
  if (!destination) return fallback;
  const hashIndex = destination.indexOf("#");
  const beforeHash = hashIndex === -1 ? destination : destination.slice(0, hashIndex);
  const rawHash = hashIndex === -1 ? undefined : destination.slice(hashIndex + 1);
  const queryIndex = beforeHash.indexOf("?");
  const rawPath = queryIndex === -1 ? beforeHash : beforeHash.slice(0, queryIndex);
  const rawQuery = queryIndex === -1 ? undefined : beforeHash.slice(queryIndex + 1);
  const fragmentOnly = beforeHash === "" && rawHash !== undefined;
  const pathname =
    rawPath === ""
      ? fallback.pathname
      : rawPath.startsWith("/")
        ? normalizePathname(rawPath)
        : normalizePathname(`${fallback.pathname}/${rawPath}`);
  const search = fragmentOnly ? fallback.search : rawQuery ? `?${rawQuery}` : "";
  const hash = rawHash ? `#${rawHash}` : "";
  return {
    href: `${pathname}${search}${hash}`,
    pathname,
    search,
    hash,
    query: parseQuery(search.startsWith("?") ? search.slice(1) : ""),
  };
}

function resolveRoutePattern(
  definition: NativeRouteDefinition,
  byId: Map<string, NativeRouteDefinition>,
  cache: Map<string, { pattern: string; routeIds: string[] }>,
): { pattern: string; routeIds: string[] } {
  const cached = cache.get(definition.id);
  if (cached) return cached;
  const parent = definition.parentId
    ? resolveRoutePattern(byId.get(definition.parentId)!, byId, cache)
    : { pattern: "/", routeIds: [] };
  let pattern = parent.pattern;
  if (definition.path !== undefined) {
    if (definition.path.startsWith("/")) pattern = normalizePathname(definition.path);
    else if (definition.path !== "") {
      pattern = normalizePathname(`${parent.pattern.replace(/\/$/, "")}/${definition.path}`);
    }
  }
  const resolved = { pattern, routeIds: [...parent.routeIds, definition.id] };
  cache.set(definition.id, resolved);
  return resolved;
}

function compileSegments(pattern: string): PatternSegment[] {
  return pattern
    .split("/")
    .filter(Boolean)
    .map((segment): PatternSegment => {
      if (segment.startsWith("*")) {
        return { kind: "wildcard", name: segment.slice(1) || "*" };
      }
      if (segment.startsWith(":")) {
        const optional = segment.endsWith("?");
        return {
          kind: "param",
          name: optional ? segment.slice(1, -1) : segment.slice(1),
          optional,
        };
      }
      return { kind: "static", value: decodeComponent(segment, false) };
    });
}

function matchSegments(
  segments: PatternSegment[],
  parts: string[],
  segmentIndex = 0,
  partIndex = 0,
  params: NativeRouteValue[] = [],
): NativeRouteValue[] | undefined {
  const segment = segments[segmentIndex];
  if (!segment) return partIndex === parts.length ? params : undefined;
  if (segment.kind === "static") {
    if (parts[partIndex] !== segment.value) return undefined;
    return matchSegments(segments, parts, segmentIndex + 1, partIndex + 1, params);
  }
  if (segment.kind === "param") {
    const value = parts[partIndex];
    if (value !== undefined) {
      params.push({ name: segment.name, value });
      const matched = matchSegments(segments, parts, segmentIndex + 1, partIndex + 1, params);
      if (matched) return matched;
      params.pop();
    }
    if (segment.optional) {
      return matchSegments(segments, parts, segmentIndex + 1, partIndex, params);
    }
    return undefined;
  }
  params.push({ name: segment.name, value: parts.slice(partIndex).join("/") });
  return params;
}

function specificityOf(segments: PatternSegment[], ancestryDepth: number): RouteSpecificity {
  let staticSegments = 0;
  let requiredSegments = 0;
  let optionalSegments = 0;
  let wildcard = false;
  for (const segment of segments) {
    if (segment.kind === "static") {
      staticSegments += 1;
      requiredSegments += 1;
    } else if (segment.kind === "param") {
      if (segment.optional) optionalSegments += 1;
      else requiredSegments += 1;
    } else wildcard = true;
  }
  return { staticSegments, requiredSegments, optionalSegments, wildcard, ancestryDepth };
}

function isMoreSpecific(left: RouteSpecificity, right: RouteSpecificity): boolean {
  if (left.staticSegments !== right.staticSegments)
    return left.staticSegments > right.staticSegments;
  if (left.requiredSegments !== right.requiredSegments) {
    return left.requiredSegments > right.requiredSegments;
  }
  if (left.wildcard !== right.wildcard) return !left.wildcard;
  if (left.optionalSegments !== right.optionalSegments) {
    return left.optionalSegments < right.optionalSegments;
  }
  return left.ancestryDepth > right.ancestryDepth;
}

function compileRoutes(definitions: NativeRouteDefinition[]): CompiledRoute[] {
  const byId = new Map(definitions.map((definition) => [definition.id, definition]));
  const cache = new Map<string, { pattern: string; routeIds: string[] }>();
  return definitions.map((definition) => {
    const { pattern, routeIds } = resolveRoutePattern(definition, byId, cache);
    const segments = compileSegments(pattern);
    return {
      id: definition.id,
      routeIds,
      segments,
      participates: definition.path !== undefined,
      specificity: specificityOf(segments, routeIds.length),
    };
  });
}

function matchRoutes(routes: CompiledRoute[], pathname: string): NativeRouterState["matched"] {
  const parts = pathname
    .split("/")
    .filter(Boolean)
    .map((segment) => decodeComponent(segment, false));
  let best: { route: CompiledRoute; params: NativeRouteValue[] } | undefined;
  for (const route of routes) {
    if (!route.participates) continue;
    const params = matchSegments(route.segments, parts);
    if (!params) continue;
    if (!best || isMoreSpecific(route.specificity, best.route.specificity)) {
      best = { route, params };
    }
  }
  return best ? { routeIds: best.route.routeIds, params: best.params } : undefined;
}

/** In-memory core router used by Solid tests that mock the native binding. */
export class NativeRouter {
  private routes: CompiledRoute[];
  private entries: NativeRouterState[] = [];
  private index = 0;

  constructor(routes: NativeRouteDefinition[], initialDestination?: string | null) {
    this.routes = compileRoutes(routes);
    this.entries = [this.snapshot(parseLocation(initialDestination ?? "/"))];
  }

  private snapshot(location: NativeRouteLocation): NativeRouterState {
    return {
      location,
      matched: matchRoutes(this.routes, location.pathname),
      historyIndex: this.index,
      historyLength: this.entries.length,
      canGoBack: this.index > 0,
      canGoForward: this.index + 1 < this.entries.length,
    };
  }

  private commit(location: NativeRouteLocation, replace: boolean): NativeRouterState {
    const next = this.snapshot(location);
    if (replace) this.entries[this.index] = next;
    else {
      this.entries = this.entries.slice(0, this.index + 1);
      this.entries.push(next);
      this.index = this.entries.length - 1;
    }
    return this.state();
  }

  state(): NativeRouterState {
    const current = this.entries[this.index]!;
    return {
      ...current,
      historyIndex: this.index,
      historyLength: this.entries.length,
      canGoBack: this.index > 0,
      canGoForward: this.index + 1 < this.entries.length,
    };
  }

  resolve(destination: string): NativeRouteLocation {
    return parseLocation(destination, this.entries[this.index]?.location);
  }

  isActive(destination: string, end = false): boolean {
    const current = this.entries[this.index]!.location.pathname;
    const target = this.resolve(destination).pathname;
    if (current === target) return true;
    if (end) return false;
    if (target === "/") return current.startsWith("/");
    return current.startsWith(`${target}/`);
  }

  push(destination: string): NativeRouterState {
    return this.commit(this.resolve(destination), false);
  }

  replace(destination: string): NativeRouterState {
    return this.commit(this.resolve(destination), true);
  }

  go(delta: number): NativeRouterState {
    this.index = Math.max(0, Math.min(this.entries.length - 1, this.index + delta));
    return this.state();
  }

  back(): NativeRouterState {
    return this.go(-1);
  }

  forward(): NativeRouterState {
    return this.go(1);
  }

  dispose(): void {}
}

fakeBinding.NativeRouter = NativeRouter;
