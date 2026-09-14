import { useEffect, useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import measured from "../../data/desktop-benchmarks.json";
import { SectionHeading } from "../section-heading";
import { site } from "../../lib/site";

const MB = 1_000_000;
const colors: Record<string, string> = {
  "quickgui-go": "var(--peach)",
  "quickgui-typescript": "#7ea6e8",
  "quickgui-rust": "#b7794d",
  gpui: "#5e6ad2",
  tauri: "#64748b",
  electron: "#71717a",
};
type Metric = "memory" | "bundle";

function axisMaximum(value: number): number {
  const magnitude = 10 ** Math.floor(Math.log10(value / 4));
  const step = [1, 2, 5, 10].find((factor) => factor * magnitude >= value / 4)! * magnitude;
  return Math.ceil(value / step) * step;
}

function MeasurementChart({ metric }: { metric: Metric }) {
  const { t, i18n } = useTranslation();
  const container = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(480);
  const id = useId();
  const number = (value: number) =>
    new Intl.NumberFormat(i18n.language, { maximumFractionDigits: 1 }).format(value);
  const rowDate = new Intl.DateTimeFormat(i18n.language, {
    month: "short",
    day: "numeric",
    timeZone: "UTC",
  });
  const valueOf = (row: (typeof measured.results)[number]) =>
    (metric === "memory" ? row.memoryBytes : row.bundleBytes) / MB;
  const maximum = axisMaximum(
    Math.max(
      ...measured.results.map((row) =>
        metric === "memory" ? row.memoryMaxBytes / MB : valueOf(row),
      ),
    ),
  );
  const left = 148;
  const chartHeight = measured.results.length * 54 + 48;
  const plotWidth = Math.max(1, width - left - 80);
  const x = (value: number) => left + (value / maximum) * plotWidth;
  const tickCount = plotWidth < 100 ? 2 : plotWidth < 200 ? 3 : 5;
  const ticks = Array.from({ length: tickCount }, (_, index) => (maximum * index) / (tickCount - 1));
  const title = t(`benchmarks.${metric}`);

  useEffect(() => {
    if (!container.current) return;
    const observer = new ResizeObserver(([entry]) => {
      if (entry && entry.contentRect.width > 0) setWidth(Math.round(entry.contentRect.width));
    });
    observer.observe(container.current);
    return () => observer.disconnect();
  }, []);

  return (
    <figure className="min-w-0 bg-background p-6 sm:p-8">
      <figcaption>
        <h3 className="text-base font-semibold">{title}</h3>
      </figcaption>
      <div ref={container} className="mt-6 w-full">
        <svg
          width="100%"
          height={chartHeight}
          viewBox={`0 0 ${width} ${chartHeight}`}
          role="img"
          aria-labelledby={`${id}-title ${id}-description`}
        >
          <title id={`${id}-title`}>{title}</title>
          <desc id={`${id}-description`}>
            {measured.results.map((row) => `${row.name}: ${number(valueOf(row))} MB`).join("; ")}
          </desc>
          {ticks.map((tick, index) => (
            <g key={index}>
              <line
                x1={x(tick)}
                x2={x(tick)}
                y1={8}
                y2={chartHeight - 34}
                stroke="var(--border)"
                strokeDasharray={index ? "3 4" : undefined}
              />
              <text
                x={x(tick)}
                y={chartHeight - 11}
                textAnchor={index === ticks.length - 1 ? "end" : index === 0 ? "start" : "middle"}
                fill="var(--muted-foreground)"
                fontSize="11"
              >
                {number(tick)}
              </text>
            </g>
          ))}
          {measured.results.map((row, index) => {
            const y = 28 + index * 54;
            const value = valueOf(row);
            const range =
              metric === "memory"
                ? ` · ${t("benchmarks.range", {
                    min: number(row.memoryMinBytes / MB),
                    max: number(row.memoryMaxBytes / MB),
                  })}`
                : "";
            return (
              <g key={row.id}>
                <title>{`${row.name} ${row.version}: ${number(value)} MB${range}`}</title>
                <text x={0} y={y + 4} fill="var(--foreground)" fontSize="12" fontWeight="500">
                  {row.name}
                </text>
                <text x={0} y={y + 20} fill="var(--muted-foreground)" fontSize="10">
                  v{row.version} · {rowDate.format(new Date(row.measuredAt))}
                </text>
                <rect
                  x={left}
                  y={y - 8}
                  width={x(value) - left}
                  height={20}
                  fill={colors[row.id] ?? "var(--muted-foreground)"}
                />
                {metric === "memory" && (
                  <g stroke="var(--foreground)" strokeWidth="1.5">
                    <line
                      x1={x(row.memoryMinBytes / MB)}
                      x2={x(row.memoryMaxBytes / MB)}
                      y1={y + 2}
                      y2={y + 2}
                    />
                    <line
                      x1={x(row.memoryMinBytes / MB)}
                      x2={x(row.memoryMinBytes / MB)}
                      y1={y - 3}
                      y2={y + 7}
                    />
                    <line
                      x1={x(row.memoryMaxBytes / MB)}
                      x2={x(row.memoryMaxBytes / MB)}
                      y1={y - 3}
                      y2={y + 7}
                    />
                  </g>
                )}
                <text
                  x={x(metric === "memory" ? row.memoryMaxBytes / MB : value) + 8}
                  y={y + 6}
                  fill="var(--foreground)"
                  fontSize="12"
                  fontWeight="500"
                >
                  {number(value)} {t("benchmarks.unit")}
                </text>
              </g>
            );
          })}
        </svg>
      </div>
    </figure>
  );
}

export function Benchmarks() {
  const { t, i18n } = useTranslation();
  const [preview, setPreview] = useState(measured.results[0]);
  const dates = measured.results.map((row) => Date.parse(row.measuredAt));
  const dateFormat = new Intl.DateTimeFormat(i18n.language, {
    dateStyle: "medium",
    timeZone: "UTC",
  });
  const date = dateFormat.formatRange(new Date(Math.min(...dates)), new Date(Math.max(...dates)));
  return (
    <section id="benchmarks" className="border-b border-border">
      <SectionHeading title={t("benchmarks.title")} lead={t("benchmarks.lead")} />
      <div className="flex flex-wrap justify-between gap-2 border-b border-border bg-card-2 px-6 py-4 font-mono text-[11px] text-muted-foreground sm:px-8">
        <span>
          {t("benchmarks.machine", {
            chip: measured.machine.chip,
            ram: measured.machine.memoryBytes / 1024 ** 3,
            os: measured.machine.osVersion,
          })}
        </span>
        <span>{t("benchmarks.measured", { date })}</span>
      </div>
      <div className="grid gap-px bg-border lg:grid-cols-2">
        <MeasurementChart metric="memory" />
        <MeasurementChart metric="bundle" />
      </div>
      <div className="space-y-4 border-t border-border px-6 py-6 text-xs leading-relaxed text-muted-foreground sm:px-8">
        <p>
          {t("benchmarks.method", {
            runs: measured.methodology.runs,
            samples: measured.methodology.samplesPerRun,
            warmup: measured.methodology.warmupMs / 1000,
          })}
        </p>
        <details className="group">
          <summary className="w-fit cursor-pointer text-foreground underline decoration-border underline-offset-4">
            {t("benchmarks.methodology")}
          </summary>
          <div className="mt-3 max-w-4xl space-y-3">
            <p>{t("benchmarks.memoryMethod")}</p>
            <p>{t("benchmarks.bundleMethod")}</p>
            <p>{t("benchmarks.scope")}</p>
          </div>
        </details>
        <details>
          <summary className="w-fit cursor-pointer text-foreground underline decoration-border underline-offset-4">
            {t("benchmarks.preview")}
          </summary>
          <div className="mt-4 overflow-hidden rounded-lg border border-border bg-card-2">
            <div
              className="flex flex-wrap gap-1 border-b border-border p-2"
              aria-label={t("benchmarks.framework")}
            >
              {measured.results.map((row) => (
                <button
                  key={row.id}
                  type="button"
                  aria-pressed={preview.id === row.id}
                  onClick={() => setPreview(row)}
                  className="rounded-md px-3 py-2 text-xs text-muted-foreground hover:text-foreground aria-pressed:bg-background aria-pressed:text-foreground aria-pressed:shadow-sm"
                >
                  {row.name}
                </button>
              ))}
            </div>
            <img
              src={`/benchmarks/${preview.id}.png?v=${encodeURIComponent(measured.measuredAt)}`}
              alt={t("benchmarks.previewAlt", { framework: preview.name })}
              width={2200}
              height={1504}
              loading="lazy"
              className="h-auto w-full"
              onError={(event) => {
                event.currentTarget.hidden = true;
              }}
            />
          </div>
        </details>
        <div className="flex flex-wrap gap-5 font-medium text-foreground">
          <a
            href="/benchmarks/desktop-macos-arm64.json"
            className="underline decoration-border underline-offset-4"
          >
            {t("benchmarks.raw")}
          </a>
          <a
            href={`${site.links.github}/tree/main/benchmarks/desktop`}
            className="underline decoration-border underline-offset-4"
          >
            {t("benchmarks.source")}
          </a>
        </div>
      </div>
    </section>
  );
}
