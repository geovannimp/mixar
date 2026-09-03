import mermaid from "mermaid";

/** atob() is Latin-1; chart source is UTF-8 base64 from the Astro build. */
export function decodeBase64Utf8(value: string): string {
  const bytes = Uint8Array.from(atob(value), (char) => char.charCodeAt(0));
  return new TextDecoder().decode(bytes);
}

const hosts = new Set<HTMLElement>();
let themeObserver: MutationObserver | undefined;

function themeConfig(): Parameters<typeof mermaid.initialize>[0] {
  const style = getComputedStyle(document.documentElement);
  const dark = document.documentElement.getAttribute("data-theme") === "dark";

  return {
    startOnLoad: false,
    theme: "base",
    themeVariables: {
      darkMode: dark,
      background: "transparent",
      mainBkg: style.getPropertyValue("--bg-elevated").trim(),
      nodeBorder: style.getPropertyValue("--border").trim(),
      clusterBkg: style.getPropertyValue("--bg-elevated").trim(),
      clusterBorder: style.getPropertyValue("--border").trim(),
      titleColor: style.getPropertyValue("--text").trim(),
      edgeLabelBackground: style.getPropertyValue("--bg-card").trim(),
      lineColor: style.getPropertyValue("--accent").trim(),
      primaryColor: style.getPropertyValue("--bg-elevated").trim(),
      primaryTextColor: style.getPropertyValue("--text").trim(),
      primaryBorderColor: style.getPropertyValue("--deck-b").trim(),
      secondaryColor: style.getPropertyValue("--accent-dim").trim(),
      tertiaryColor: style.getPropertyValue("--bg-card").trim(),
      fontFamily: style.getPropertyValue("--font-sans").trim(),
    },
    flowchart: {
      htmlLabels: true,
      useMaxWidth: false,
      curve: "basis",
      subGraphTitleMargin: {
        top: 12,
        bottom: 10,
      },
    },
    securityLevel: "loose",
  };
}

async function renderHost(host: HTMLElement, chart: string): Promise<void> {
  mermaid.initialize(themeConfig());

  const id = `mmd-${Math.random().toString(36).slice(2)}`;
  const { svg, bindFunctions } = await mermaid.render(id, chart);
  host.innerHTML = svg;
  bindFunctions?.(host);
  host.removeAttribute("aria-busy");

  const svgEl = host.querySelector("svg");
  if (!svgEl) return;

  svgEl.style.height = "clamp(18rem, 42vw, 24rem)";
  svgEl.style.width = "auto";
  svgEl.style.maxWidth = "none";
}

async function renderAll(): Promise<void> {
  await Promise.all(
    [...hosts].map((host) => {
      const chart = host.dataset.mermaidChart;
      if (!chart) return Promise.resolve();
      return renderHost(host, chart);
    }),
  );
}

function ensureThemeObserver(): void {
  if (themeObserver) return;

  themeObserver = new MutationObserver(() => {
    void renderAll();
  });
  themeObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-theme"],
  });
}

export function mountMermaidDiagram(host: HTMLElement, chart: string): void {
  host.dataset.mermaidChart = chart;
  host.setAttribute("aria-busy", "true");
  hosts.add(host);
  ensureThemeObserver();
  void renderHost(host, chart);
}
