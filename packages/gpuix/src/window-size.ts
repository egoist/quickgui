import { useSyncExternalStore } from "react";
import { useGpuix } from "@gpuix/react";
import type { GpuixRenderer } from "./native.ts";
export function useWindowSize() {
  const { renderer } = useGpuix();
  const host = renderer as unknown as GpuixRenderer;
  return useSyncExternalStore(
    (listener) => host.subscribeWindowSize(listener),
    () => host.getWindowSize(),
  );
}
export function useWindowInsets() {
  const size = useWindowSize();
  const edges = { top: 0, right: 0, bottom: 0, left: 0 };
  return {
    safeArea: edges,
    ime: edges,
    effective: edges,
    keyboardTop: size.height,
    keyboardVisible: false,
    visibleHeight: size.height,
  };
}
