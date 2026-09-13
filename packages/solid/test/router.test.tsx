import { expect, test } from "bun:test";
import { flush } from "solid-js";
import { app, NativeNodeTag, PropertyCode, Window, type NativeNode } from "@quickgui/native";
import { Button, Text, View, createRenderer } from "../src/index.ts";
import {
  Link,
  Outlet,
  Route,
  Router,
  useLocation,
  useNavigate,
  useParams,
  useSearchParams,
  type RouteSectionProps,
} from "../src/router.ts";

await app.whenReady();

function text(node: NativeNode): string {
  return (node.text ?? "") + node.children.map(text).join("");
}

test("useParams, useLocation, and useSearchParams return reactive objects", () => {
  const host = new Window({ renderer: () => () => {} });
  let constructions = 0;
  let params: ReturnType<typeof useParams<{ id: string }>> | undefined;
  let location: ReturnType<typeof useLocation> | undefined;
  let searchParams: ReturnType<typeof useSearchParams<{ tab: string; token: string }>> | undefined;
  let navigate: ReturnType<typeof useNavigate> | undefined;

  function Project() {
    constructions += 1;
    params = useParams<{ id: string }>();
    location = useLocation();
    searchParams = useSearchParams<{ tab: string; token: string }>();
    navigate = useNavigate();
    return (
      <Text>
        Project {params.id} at {location.pathname} tab {searchParams.tab ?? ""}
      </Text>
    );
  }

  const dispose = createRenderer(() => (
    <Router initialPath="/projects/12">
      <Route path="/projects/:id" component={Project} />
    </Router>
  ))(host);

  expect(typeof params).toBe("object");
  expect(typeof params).not.toBe("function");
  expect(typeof location).toBe("object");
  expect(typeof location).not.toBe("function");
  expect(typeof searchParams).toBe("object");
  expect(typeof searchParams).not.toBe("function");
  expect(params?.id).toBe("12");
  expect(location?.pathname).toBe("/projects/12");
  expect(text(host.root)).toContain("Project 12 at /projects/12");
  expect(constructions).toBe(1);

  navigate!("/projects/13?tab=activity");
  flush();
  expect(constructions).toBe(1);
  expect(params?.id).toBe("13");
  expect(location?.pathname).toBe("/projects/13");
  expect(location?.search).toBe("?tab=activity");
  expect(searchParams?.tab).toBe("activity");
  expect(text(host.root)).toContain("Project 13 at /projects/13 tab activity");

  navigate!("#details");
  flush();
  expect(constructions).toBe(1);
  expect(location?.pathname).toBe("/projects/13");
  expect(location?.search).toBe("?tab=activity");
  expect(location?.hash).toBe("#details");

  navigate!("/projects/Quick%20GUI?token=a=b");
  flush();
  expect(constructions).toBe(1);
  expect(params?.id).toBe("Quick GUI");
  expect(searchParams?.token).toBe("a=b");
  expect(location?.pathname).toBe("/projects/Quick%20GUI");

  dispose();
  host.close();
});

test("route section props expose the same reactive objects and keep layouts mounted", () => {
  const host = new Window({ renderer: () => () => {} });
  let shellCreated = 0;
  let projectCreated = 0;
  let navigate: ReturnType<typeof useNavigate> | undefined;

  function Shell(props: RouteSectionProps) {
    shellCreated += 1;
    navigate = useNavigate();
    return (
      <View>
        <Text>shell {props.location.pathname}</Text>
        {props.children}
      </View>
    );
  }

  function Project(props: RouteSectionProps) {
    projectCreated += 1;
    return (
      <Text>
        project {props.params.id} {props.searchParams.tab ?? ""}
      </Text>
    );
  }

  const dispose = createRenderer(() => (
    <Router initialPath="/projects/12">
      <Route path="/" component={Shell}>
        <Route path="/projects/:id" component={Project} />
      </Route>
    </Router>
  ))(host);

  expect(shellCreated).toBe(1);
  expect(projectCreated).toBe(1);
  expect(text(host.root)).toContain("shell /projects/12");
  expect(text(host.root)).toContain("project 12");

  navigate!("/projects/13?tab=history");
  flush();
  expect(shellCreated).toBe(1);
  expect(projectCreated).toBe(1);
  expect(text(host.root)).toContain("shell /projects/13");
  expect(text(host.root)).toContain("project 13 history");

  dispose();
  host.close();
});

test("Link navigates through the current router and marks the role", () => {
  const host = new Window({ renderer: () => () => {} });

  function Home() {
    return <Link href="/settings">Settings</Link>;
  }

  function Settings() {
    return <Text>Settings</Text>;
  }

  const dispose = createRenderer(() => (
    <Router initialPath="/">
      <Route path="/" component={Home} />
      <Route path="/settings" component={Settings} />
    </Router>
  ))(host);

  const link = [...host.nodes.values()].find((node) => node.tag === NativeNodeTag.Button);
  expect(link).toBeDefined();
  expect(link!.properties.get(PropertyCode.Role)).toBe("link");
  host._dispatchEvent("click", link!.id);
  flush();
  expect(text(host.root)).toContain("Settings");

  dispose();
  host.close();
});

test("Outlet renders the next matched child and fallback when unmatched", () => {
  const host = new Window({ renderer: () => () => {} });
  let navigate: ReturnType<typeof useNavigate> | undefined;

  function Shell() {
    navigate = useNavigate();
    return (
      <View>
        <Text>chrome</Text>
        <Outlet />
      </View>
    );
  }

  const dispose = createRenderer(() => (
    <Router initialPath="/" fallback={<Text>missing</Text>}>
      <Route path="/" component={Shell}>
        <Route path="/" component={() => <Text>home</Text>} />
        <Route path="/about" component={() => <Text>about</Text>} />
      </Route>
    </Router>
  ))(host);

  expect(text(host.root)).toContain("chrome");
  expect(text(host.root)).toContain("home");
  navigate!("/about");
  flush();
  expect(text(host.root)).toContain("about");
  navigate!("/missing");
  flush();
  expect(text(host.root)).toContain("missing");

  dispose();
  host.close();
});
