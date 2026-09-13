import { useEffect, useRef, useState } from "react";
import type { ComponentDoc } from "../../lib/component-docs";
import type { DemoSource } from "../../lib/demo-source";
import type { Locale } from "../../i18n";
import { DEMO_COMPONENTS } from "../../lib/component-demos";

const labels = {
  en: {
    preview: "Preview",
    code: "Code",
    reset: "Reset demo",
    open: "Open demo",
    loading: "Loading interactive demo…",
    retry: "Try again",
    failed: "The demo could not start. Use a browser with WebGPU or WebGL2 enabled, then try again.",
  },
  zh: {
    preview: "预览",
    code: "代码",
    reset: "重置演示",
    open: "打开演示",
    loading: "正在加载交互演示…",
    retry: "重试",
    failed: "演示无法启动。请使用启用 WebGPU 或 WebGL2 的浏览器后重试。",
  },
  ja: {
    preview: "プレビュー",
    code: "コード",
    reset: "デモをリセット",
    open: "デモを開く",
    loading: "デモを読み込み中…",
    retry: "再試行",
    failed: "デモを開始できませんでした。WebGPU または WebGL2 が有効なブラウザで再試行してください。",
  },
};
export function ComponentPreview({
  component,
  source,
  locale,
}: {
  component: ComponentDoc;
  source: DemoSource;
  locale: Locale;
}) {
  const [tab, setTab] = useState<"preview" | "code">("preview");
  const [revision, setRevision] = useState(0);
  const [status, setStatus] = useState<"loading" | "ready" | "error">("loading");
  const frame = useRef<HTMLIFrameElement>(null);
  const text = labels[locale];
  const available = component.kind === "ui" && DEMO_COMPONENTS.includes(component.slug);
  const url = `/demos/index.html?component=${encodeURIComponent(component.slug)}`;
  useEffect(() => {
    setStatus("loading");
    if (!available) return;
    function receive(event: MessageEvent) {
      if (
        event.origin !== window.location.origin ||
        event.source !== frame.current?.contentWindow ||
        event.data?.type !== "quickgui-demo"
      )
        return;
      if (event.data.status === "ready" || event.data.status === "error") {
        setStatus(event.data.status);
        clearTimeout(timeout);
      }
    }
    const timeout = setTimeout(() => setStatus("error"), 45_000);
    window.addEventListener("message", receive);
    frame.current?.contentWindow?.postMessage(
      { type: "quickgui-demo-status-request" },
      window.location.origin,
    );
    return () => {
      clearTimeout(timeout);
      window.removeEventListener("message", receive);
    };
  }, [available, component.slug, revision]);
  if (!available) return null;
  return (
    <section
      className="component-preview"
      id="preview"
      aria-label={`${component.name} ${text.preview}`}
    >
      <div className="component-preview-toolbar">
        <div role="tablist" aria-label={component.name}>
          {(["preview", "code"] as const).map((name) => (
            <button
              key={name}
              type="button"
              role="tab"
              id={`demo-tab-${name}`}
              aria-controls={`demo-panel-${name}`}
              aria-selected={tab === name}
              tabIndex={tab === name ? 0 : -1}
              onClick={() => setTab(name)}
              onKeyDown={(event) => {
                if (["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) {
                  event.preventDefault();
                  const next =
                    event.key === "Home"
                      ? "preview"
                      : event.key === "End"
                        ? "code"
                        : tab === "preview"
                          ? "code"
                          : "preview";
                  setTab(next);
                  document.getElementById(`demo-tab-${next}`)?.focus();
                }
              }}
            >
              {text[name]}
            </button>
          ))}
        </div>
        <div className="component-preview-actions">
          {available && (
            <>
              <button
                type="button"
                aria-label={text.reset}
                title={text.reset}
                onClick={() => setRevision((value) => value + 1)}
              >
                <span className="i-lucide-rotate-ccw" aria-hidden />
              </button>
              <a
                href={url}
                aria-label={text.open}
                title={text.open}
                target="_blank"
                rel="noreferrer"
              >
                <span className="i-lucide-external-link" aria-hidden />
              </a>
            </>
          )}
        </div>
      </div>
      <div
        id="demo-panel-preview"
        role="tabpanel"
        aria-labelledby="demo-tab-preview"
        hidden={tab !== "preview"}
        className="component-preview-stage"
      >
        {available ? (
          <>
            <iframe
              key={`${component.slug}-${revision}`}
              ref={frame}
              title={`${component.name} interactive demo`}
              src={url}
              loading="lazy"
              sandbox="allow-scripts allow-same-origin allow-pointer-lock"
              allow="gpu; webgpu"
              tabIndex={status === "ready" ? 0 : -1}
            />
            {status !== "ready" && (
              <div className="component-preview-status" role="status">
                <span
                  className={
                    status === "loading"
                      ? "i-lucide-loader-circle component-preview-spinner"
                      : "i-lucide-monitor"
                  }
                  aria-hidden
                />
                <p>{status === "loading" ? text.loading : text.failed}</p>
                {status === "error" && (
                  <button type="button" onClick={() => setRevision((value) => value + 1)}>
                    {text.retry}
                  </button>
                )}
              </div>
            )}
          </>
        ) : null}
      </div>
      <div
        id="demo-panel-code"
        role="tabpanel"
        aria-labelledby="demo-tab-code"
        hidden={tab !== "code"}
        className="component-preview-code"
      >
        {/* Trusted HTML generated by Shiki from the repository’s own examples. */}
        <div className="docs-code-source" dangerouslySetInnerHTML={{ __html: source.html }} />
      </div>
    </section>
  );
}
