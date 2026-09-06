import { describe, expect, test } from "bun:test";

import { goBuildArgs, goTargetEnv, hostLibraryName } from "./go-build.ts";

describe("Go compilation mapping", () => {
  test("maps QuickGUI targets onto GOOS and GOARCH", () => {
    expect(goTargetEnv("darwin-arm64")).toEqual({ GOOS: "darwin", GOARCH: "arm64" });
    expect(goTargetEnv("darwin-x64")).toEqual({ GOOS: "darwin", GOARCH: "amd64" });
    expect(goTargetEnv("linux-x64")).toEqual({ GOOS: "linux", GOARCH: "amd64" });
    expect(goTargetEnv("windows-arm64")).toEqual({ GOOS: "windows", GOARCH: "arm64" });
  });

  test("names the staged host shared library for each platform", () => {
    expect(hostLibraryName("darwin")).toBe("libquickgui_host.dylib");
    expect(hostLibraryName("linux")).toBe("libquickgui_host.so");
    expect(hostLibraryName("windows")).toBe("quickgui_host.dll");
  });

  test("production builds strip symbols and hide the Windows console", () => {
    expect(
      goBuildArgs({
        mode: "production",
        hideConsole: true,
        platform: "windows",
        output: "/out/App.exe",
        packageArg: ".",
      }),
    ).toEqual(["build", "-o", "/out/App.exe", "-ldflags", "-s -w -H windowsgui", "."]);
    expect(
      goBuildArgs({
        mode: "development",
        hideConsole: false,
        platform: "darwin",
        output: "/out/App",
        packageArg: ".",
      }),
    ).toEqual(["build", "-o", "/out/App", "."]);
  });
});
