import {
  Router as CoreRouter,
  type QuickGuiEvent,
  type RouteDefinition as CoreRouteDefinition,
  type RouteLocation,
  type RouterState,
} from "@quickgui/native";
import {
  createContext,
  createMemo,
  createSignal,
  flush as flushSolid,
  getOwner,
  omit,
  runWithOwner,
  untrack,
  useContext,
  onCleanup,
  type Accessor,
} from "solid-js";

import { Button, createComponent, mergeProps, type JSX } from "./index.ts";

const routeDefinition = Symbol("QuickGUI route definition");

export type RouteParams = Readonly<Record<string, string | undefined>>;
export type RouteSearchParams = Readonly<Record<string, string | readonly string[] | undefined>>;

export interface RouteSectionProps {
  /** Reactive object of decoded pattern parameters. Read a property in JSX to track it. */
  readonly params: RouteParams;
  /** Reactive object for the normalized current location. */
  readonly location: RouteLocation;
  /** Reactive object of decoded query values. Repeated names are arrays. */
  readonly searchParams: RouteSearchParams;
  /** The next matched child route. A layout may render this or use `<Outlet />`. */
  readonly children: JSX.Element;
}

export type RouteComponent = (props: RouteSectionProps) => JSX.Element;

export interface RouteProps {
  /** Stable declaration identity. QuickGUI generates one when omitted. */
  id?: string;
  /** Absolute or parent-relative core route pattern. Omit it for a pathless layout. */
  path?: string;
  component?: RouteComponent;
  children?: JSX.Element;
}

interface SolidRouteDefinition {
  readonly [routeDefinition]: true;
  readonly id: string | undefined;
  readonly path: string | undefined;
  readonly component: RouteComponent | undefined;
  readonly children: () => unknown;
}

interface RenderedRoute {
  id: string;
  component: RouteComponent | undefined;
}

interface RouteTable {
  definitions: CoreRouteDefinition[];
  chains: Map<string, readonly RenderedRoute[]>;
}

/** Declare one route. Definitions are consumed by the nearest `<Router>` and render no node. */
export function Route(props: RouteProps): JSX.Element {
  return {
    [routeDefinition]: true,
    get id() {
      return props.id;
    },
    get path() {
      return props.path;
    },
    get component() {
      return props.component;
    },
    children: () => props.children,
  } as SolidRouteDefinition as unknown as JSX.Element;
}

export interface NavigateOptions {
  replace?: boolean;
}

export interface RouterController {
  /** Current core snapshot as a Solid accessor. */
  readonly state: Accessor<RouterState>;
  navigate(destination: string, options?: NavigateOptions): void;
  push(destination: string): void;
  replace(destination: string): void;
  go(delta: number): void;
  back(): void;
  forward(): void;
  resolve(destination: string): RouteLocation;
  isActive(destination: string, end?: boolean): boolean;
}

interface RouterContextValue {
  controller: RouterController;
  state: Accessor<RouterState>;
  params: RouteParams;
  location: RouteLocation;
  searchParams: RouteSearchParams;
}

const RouterContext = createContext<RouterContextValue | undefined>();
const OutletContext = createContext<(() => JSX.Element) | undefined>();

export interface RouterProps {
  /** Initial memory-history destination. Defaults to `/`. */
  initialPath?: string;
  /** Rendered when no declared route matches. */
  fallback?: JSX.Element;
  children?: JSX.Element;
}

/**
 * Solid projection of QuickGUI's core route table and bounded memory history.
 *
 * Route declarations are static for the lifetime of this component. Location, parameters, query,
 * history traversal, and active-link decisions all come from the Rust core.
 */
export function Router(props: RouterProps): JSX.Element {
  const table = untrack(() => buildRouteTable(props.children));
  const native = new CoreRouter(table.definitions, props.initialPath);
  onCleanup(() => native.dispose());
  const [state, setState] = createSignal<RouterState>(native.state());

  const commit = (next: RouterState) => {
    setState(next);
    // Solid 2 batches writes entering from host callbacks. Navigation may also happen from an
    // asynchronous application task, so close this external update boundary here as well.
    flushSolid();
  };
  const controller: RouterController = {
    state,
    navigate(destination, options) {
      commit(options?.replace ? native.replace(destination) : native.push(destination));
    },
    push(destination) {
      commit(native.push(destination));
    },
    replace(destination) {
      commit(native.replace(destination));
    },
    go(delta) {
      commit(native.go(delta));
    },
    back() {
      commit(native.back());
    },
    forward() {
      commit(native.forward());
    },
    resolve(destination) {
      return native.resolve(destination);
    },
    isActive(destination, end) {
      state();
      return native.isActive(destination, end);
    },
  };
  const params = createMemoObject<RouteParams>(() => valuesRecord(state().matched?.params ?? []));
  const location = createReactiveLocation(() => state().location);
  const searchParams = createMemoObject<RouteSearchParams>(() =>
    queryRecord(state().location.query),
  );
  const context: RouterContextValue = {
    controller,
    state,
    params,
    location,
    searchParams,
  };

  return RouterContext({
    value: context,
    get children() {
      return MatchedRoutes({
        table,
        get fallback() {
          return props.fallback;
        },
      });
    },
  }) as unknown as JSX.Element;
}

/** Render the next matched child route inside a layout component. */
export function Outlet(): JSX.Element {
  return (useContext(OutletContext)?.() ?? undefined) as JSX.Element;
}

/** Access the current router. Must be called below `<Router>`. */
export function useRouter(): RouterController {
  return requireRouter("useRouter").controller;
}

/** Reactive location object. Read `location.pathname` in JSX to stay current. */
export function useLocation(): RouteLocation {
  return requireRouter("useLocation").location;
}

/** Reactive parameter object, like Solid Router. Read `params.id` in JSX to stay current. */
export function useParams<T extends RouteParams = RouteParams>(): T {
  return requireRouter("useParams").params as T;
}

/** Reactive query object. Repeated names are arrays. Read `searchParams.tab` in JSX to stay current. */
export function useSearchParams<T extends RouteSearchParams = RouteSearchParams>(): T {
  return requireRouter("useSearchParams").searchParams as T;
}

/** Stable imperative navigation function for the current router. */
export function useNavigate(): RouterController["navigate"] {
  const navigate = requireRouter("useNavigate").controller.navigate;
  return navigate;
}

export interface LinkProps extends Omit<JSX.NativeProps, "onClick" | "role"> {
  href: string;
  replace?: boolean;
  /** Require an exact pathname for active styling. */
  end?: boolean;
  activeStyle?: JSX.StyleProp;
  inactiveStyle?: JSX.StyleProp;
  onClick?: (event: QuickGuiEvent) => void;
}

/** Native link button backed by the current core router. */
export function Link(props: LinkProps): JSX.Element {
  const router = useRouter();
  const rest = omit(props, "href", "replace", "end", "activeStyle", "inactiveStyle", "onClick");
  return createComponent(
    Button,
    mergeProps(rest, {
      role: "link",
      get style() {
        const conditional = router.isActive(props.href, props.end)
          ? props.activeStyle
          : props.inactiveStyle;
        // A style array merges left to right, so the active style layers over the base one and a
        // link remains interactive even when rendered inside a custom title-bar drag region.
        return [props.style, conditional, { appRegion: "no-drag" }] as JSX.StyleProp;
      },
      onClick(event: QuickGuiEvent) {
        props.onClick?.(event);
        if (!event.defaultPrevented) {
          router.navigate(
            props.href,
            props.replace === undefined ? undefined : { replace: props.replace },
          );
        }
      },
    }) as JSX.NativeProps,
  ) as JSX.Element;
}

function requireRouter(component: string): RouterContextValue {
  const context = useContext(RouterContext);
  if (!context) {
    throw new TypeError(`${component} must be used inside <Router>`);
  }
  return context;
}

function MatchedRoutes(props: { table: RouteTable; fallback?: JSX.Element }): JSX.Element {
  const context = requireRouter("Router");
  // Equality on this scalar keeps a page mounted while only its params, query, or fragment
  // changes. Those values still update through the reactive objects passed to the component.
  const leafRoute = createMemo(() => context.state().matched?.routeIds.at(-1));
  return createMemo<JSX.Element>(() => {
    const leaf = leafRoute();
    if (!leaf) return props.fallback;
    const chain = props.table.chains.get(leaf);
    if (!chain) {
      throw new Error(`QuickGUI core returned unknown route \`${leaf}\``);
    }
    return renderRouteChain(chain, 0, context);
  }) as unknown as JSX.Element;
}

function renderRouteChain(
  chain: readonly RenderedRoute[],
  index: number,
  context: RouterContextValue,
): JSX.Element {
  const route = chain[index];
  if (!route) return undefined as JSX.Element;
  const outlet = () =>
    index + 1 < chain.length
      ? renderRouteChain(chain, index + 1, context)
      : (undefined as JSX.Element);
  return OutletContext({
    value: outlet,
    get children() {
      if (!route.component) return outlet();
      const routeProps: RouteSectionProps = {
        params: context.params,
        location: context.location,
        searchParams: context.searchParams,
        get children() {
          return outlet();
        },
      };
      return untrack(() => route.component!(routeProps));
    },
  }) as unknown as JSX.Element;
}

function buildRouteTable(children: unknown): RouteTable {
  const roots = collectRouteDefinitions(children);
  const definitions: CoreRouteDefinition[] = [];
  const chains = new Map<string, readonly RenderedRoute[]>();
  let generatedId = 0;

  const visit = (
    declaration: SolidRouteDefinition,
    parentId: string | undefined,
    parentChain: readonly RenderedRoute[],
  ) => {
    const id = declaration.id ?? `quickgui-route-${generatedId}`;
    generatedId += 1;
    const rendered: RenderedRoute = {
      id,
      component: declaration.component,
    };
    const chain = [...parentChain, rendered];
    chains.set(id, chain);
    definitions.push({
      id,
      ...(declaration.path === undefined ? {} : { path: declaration.path }),
      ...(parentId === undefined ? {} : { parentId }),
    });
    for (const child of collectRouteDefinitions(declaration.children())) {
      visit(child, id, chain);
    }
  };

  for (const root of roots) visit(root, undefined, []);
  return { definitions, chains };
}

function collectRouteDefinitions(value: unknown): SolidRouteDefinition[] {
  if (value === null || value === undefined || typeof value === "boolean") {
    return [];
  }
  if (typeof value === "function") {
    return collectRouteDefinitions(value());
  }
  if (Array.isArray(value)) {
    return value.flatMap(collectRouteDefinitions);
  }
  if (isRouteDefinition(value)) return [value];
  throw new TypeError("<Router> children must be <Route> declarations");
}

function isRouteDefinition(value: unknown): value is SolidRouteDefinition {
  return (
    typeof value === "object" &&
    value !== null &&
    routeDefinition in value &&
    (value as SolidRouteDefinition)[routeDefinition] === true
  );
}

function valuesRecord(values: readonly { name: string; value: string }[]): RouteParams {
  const result = Object.create(null) as Record<string, string>;
  for (const value of values) result[value.name] = value.value;
  return Object.freeze(result);
}

function queryRecord(values: readonly { name: string; value: string }[]): RouteSearchParams {
  const grouped = new Map<string, string[]>();
  for (const value of values) {
    const current = grouped.get(value.name);
    if (current) current.push(value.value);
    else grouped.set(value.name, [value.value]);
  }
  const result = Object.create(null) as Record<string, string | readonly string[]>;
  for (const [name, entries] of grouped) {
    result[name] = entries.length === 1 ? entries[0]! : Object.freeze(entries.slice());
  }
  return Object.freeze(result);
}

/** Store-like object whose property reads track the latest source snapshot, matching Solid Router. */
function createMemoObject<T extends object>(source: Accessor<T>): T {
  const map = new Map<PropertyKey, Accessor<unknown>>();
  const owner = getOwner();
  if (!owner) {
    throw new TypeError("router hooks must be used in a reactive scope");
  }
  return new Proxy({} as T, {
    get(_, property) {
      if (!map.has(property)) {
        runWithOwner(owner, () => {
          map.set(
            property,
            createMemo(() => Reflect.get(source(), property)),
          );
        });
      }
      return map.get(property)!();
    },
    getOwnPropertyDescriptor() {
      return {
        enumerable: true,
        configurable: true,
      };
    },
    ownKeys() {
      return Reflect.ownKeys(source());
    },
    has(_, property) {
      return property in (source() as object);
    },
  });
}

function createReactiveLocation(source: Accessor<RouteLocation>): RouteLocation {
  return {
    get href() {
      return source().href;
    },
    get pathname() {
      return source().pathname;
    },
    get search() {
      return source().search;
    },
    get hash() {
      return source().hash;
    },
    get query() {
      return source().query;
    },
  };
}
