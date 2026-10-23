import type { SearchIndex, SearchResult } from "../lib/docs-search";
import type { SearchScope } from "../lib/docs-fuse-search";

type WorkerResponse =
  | { type: "ready" }
  | { type: "results"; id: number; results: SearchResult[] }
  | { type: "error"; id?: number; message: string };

const INDEX_URL = "/docs/search-index.json";

const EMPTY_HINT = "Search the current guide scope, or switch between Users and Developers.";


/**
 * Docs search dialog: fetches the static text index on first use and runs
 * typo-tolerant token search in a Web Worker.
 */
export function initDocsSearch(): void {
  const trigger = document.getElementById("docs-search-open");
  const dialog = document.getElementById("docs-search-dialog");
  const input = document.getElementById("docs-search-input");
  const list = document.getElementById("docs-search-results");
  const empty = document.getElementById("docs-search-empty");
  const status = document.getElementById("docs-search-status");
  const shortcut = document.getElementById("docs-search-shortcut");
  const scopeButtons = Array.from(
    document.querySelectorAll<HTMLButtonElement>("[data-docs-search-scope]"),
  );

  if (
    !(trigger instanceof HTMLButtonElement) ||
    !(dialog instanceof HTMLDialogElement) ||
    !(input instanceof HTMLInputElement) ||
    !(list instanceof HTMLUListElement) ||
    !(empty instanceof HTMLElement) ||
    !(status instanceof HTMLElement) ||
    scopeButtons.length === 0
  ) {
    return;
  }
  const initialScope = scopeButtons.find(
    (button) => button.getAttribute("aria-pressed") === "true",
  )?.dataset.docsSearchScope;
  if (initialScope !== "users" && initialScope !== "developers") return;

  if (shortcut !== null && /Mac|iPhone|iPad/.test(navigator.userAgent)) {
    shortcut.textContent = "⌘K";
  }

  let loading: Promise<SearchIndex> | null = null;
  let results: SearchResult[] = [];
  let active = -1;
  let scope: SearchScope = initialScope;
  let worker: Worker | null = null;
  let workerReady = false;
  let workerLoading: Promise<void> | null = null;
  let queryToken = 0;

  const setEmpty = (message: string) => {
    empty.textContent = message;
    empty.hidden = false;
  };

  const setExpanded = (expanded: boolean) => {
    input.setAttribute("aria-expanded", String(expanded));
    if (!expanded) input.removeAttribute("aria-activedescendant");
  };

  const loadIndex = (): Promise<SearchIndex> => {
    loading ??= fetch(INDEX_URL, { headers: { accept: "application/json" } })
      .then(async (response) => {
        if (!response.ok) throw new Error(`search index: HTTP ${response.status}`);
        return (await response.json()) as SearchIndex;
      })
      .catch((error: unknown) => {
        loading = null;
        throw error;
      });
    return loading;
  };

  const run = () => {
    const query = input.value.trim();
    const id = ++queryToken;
    if (query === "") {
      results = [];
      active = -1;
      render(query);
      return;
    }
    if (!workerReady || worker === null) {
      status.textContent = "Loading search…";
      void startWorker().catch((error: unknown) => {
        if (id !== queryToken) return;
        status.textContent = "";
        setEmpty("Search is unavailable right now. Try your query again.");
        console.error(error);
      });
      return;
    }
    status.textContent = "Searching…";
    worker.postMessage({ type: "search", id, query, scope });
  };

  const startWorker = (): Promise<void> => {
    if (workerReady) return Promise.resolve();
    workerLoading ??= loadIndex()
      .then((loaded) => {
        let resolveReady: () => void = () => {};
        let rejectReady: (reason?: unknown) => void = () => {};
        const ready = new Promise<void>((resolve, reject) => {
          resolveReady = resolve;
          rejectReady = reject;
        });
        const instance = new Worker(new URL("../workers/docs-search.worker.ts", import.meta.url), {
          type: "module",
        });
        worker = instance;
        instance.addEventListener("message", (event: MessageEvent<WorkerResponse>) => {
          const message = event.data;
          if (message.type === "ready") {
            workerReady = true;
            resolveReady();
            run();
            return;
          }
          if (message.type === "error") {
            if (message.id !== undefined && message.id !== queryToken) return;
            if (message.id === undefined) {
              instance.terminate();
              worker = null;
              workerReady = false;
              workerLoading = null;
              rejectReady(new Error(message.message));
            } else {
              status.textContent = "";
              setEmpty("Search is unavailable right now.");
            }
            return;
          }
          if (message.id !== queryToken) return;
          results = message.results;
          active = results.length > 0 ? 0 : -1;
          render(input.value.trim());
        });
        instance.addEventListener("error", (event) => {
          const wasReady = workerReady;
          instance.terminate();
          if (worker === instance) {
            worker = null;
            workerReady = false;
            workerLoading = null;
          }
          rejectReady(new Error(event.message));
          if (wasReady) {
            status.textContent = "";
            setEmpty("Search worker stopped. Reload to try again.");
          }
        });
        instance.postMessage({ type: "index", documents: loaded.documents });
        return ready;
      })
      .catch((error: unknown) => {
        workerLoading = null;
        throw error;
      });
    return workerLoading;
  };

  const open = () => {
    queryToken++;
    if (!dialog.open) dialog.showModal();
    input.value = "";
    results = [];
    active = -1;
    setExpanded(false);
    setEmpty(EMPTY_HINT);
    status.textContent = "Loading search…";
    list.replaceChildren();
    input.focus();
    void startWorker().catch((error: unknown) => {
      status.textContent = "";
      setEmpty("Search is unavailable right now. Reload the page to try again.");
      console.error(error);
    });
  };

  const render = (query: string) => {
    list.replaceChildren();
    empty.hidden = true;

    if (query === "") {
      setEmpty(EMPTY_HINT);
      status.textContent = "";
      setExpanded(false);
      return;
    }

    if (results.length === 0) {
      setEmpty(`No pages match “${query}”.`);
      status.textContent = `No results for ${query}`;
      setExpanded(false);
      return;
    }

    for (const [position, result] of results.entries()) {
      list.append(renderResult(result, position));
    }
    status.textContent = `${results.length} result${results.length === 1 ? "" : "s"}`;
    setExpanded(true);
    setActive(active, false);
  };

  const renderResult = (result: SearchResult, position: number): HTMLLIElement => {
    const item = document.createElement("li");
    item.id = `docs-search-result-${position}`;
    item.className = "docs-search-result";
    item.setAttribute("role", "option");
    item.setAttribute("aria-selected", String(position === active));

    const link = document.createElement("a");
    link.className = "docs-search-result-link";
    link.href = resultHref(result);

    const meta = document.createElement("span");
    meta.className = "docs-search-result-meta";
    meta.textContent = `${result.document.category} · ${result.document.topic}`;
    link.append(meta);

    const title = document.createElement("span");
    title.className = "docs-search-result-title";
    title.textContent = result.document.title;
    if (result.document.section !== undefined) {
      const section = document.createElement("span");
      section.className = "docs-search-result-section";
      section.textContent = ` — ${result.document.section}`;
      title.append(section);
    }
    link.append(title);

    if (result.snippet.length > 0) {
      const snippet = document.createElement("span");
      snippet.className = "docs-search-result-snippet";
      for (const part of result.snippet) {
        if (part.match) {
          const mark = document.createElement("mark");
          mark.textContent = part.text;
          snippet.append(mark);
        } else {
          snippet.append(part.text);
        }
      }
      link.append(snippet);
    }

    item.append(link);
    item.addEventListener("mousemove", () => setActive(position));
    return item;
  };

  const resultHref = (result: SearchResult): string =>
    result.document.anchor === undefined
      ? result.document.path
      : `${result.document.path}#${result.document.anchor}`;

  const setActive = (position: number, scroll = true) => {
    active = position;
    const items = list.querySelectorAll<HTMLLIElement>(".docs-search-result");
    items.forEach((item, position_) => {
      item.setAttribute("aria-selected", String(position_ === position));
    });
    const current = items[position];
    if (current === undefined) {
      input.removeAttribute("aria-activedescendant");
      return;
    }
    input.setAttribute("aria-activedescendant", current.id);
    if (scroll) current.scrollIntoView({ block: "nearest" });
  };

  trigger.addEventListener("click", open);

  for (const button of scopeButtons) {
    button.setAttribute("aria-pressed", String(button.dataset.docsSearchScope === scope));
    button.addEventListener("click", () => {
      const selected = button.dataset.docsSearchScope;
      if (selected !== "users" && selected !== "developers") return;
      scope = selected;
      for (const scopeButton of scopeButtons) {
        scopeButton.setAttribute("aria-pressed", String(scopeButton.dataset.docsSearchScope === scope));
      }
      results = [];
      active = -1;
      run();
    });
  }
  input.addEventListener("input", run);

  input.addEventListener("keydown", (event) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      if (results.length === 0) return;
      event.preventDefault();
      const step = event.key === "ArrowDown" ? 1 : -1;
      setActive((active + step + results.length) % results.length);
      return;
    }
    if (event.key === "Enter") {
      const result = results[active];
      if (result === undefined) return;
      event.preventDefault();
      window.location.assign(resultHref(result));
    }
  });

  dialog.addEventListener("click", (event) => {
    // A click on the backdrop targets the dialog itself; its contents do not.
    if (event.target === dialog) dialog.close();
  });

  dialog.addEventListener("close", () => {
    queryToken++;
    results = [];
    active = -1;
  });

  document.addEventListener("keydown", (event) => {
    if (event.key.toLowerCase() !== "k" || !(event.metaKey || event.ctrlKey)) return;
    event.preventDefault();
    if (dialog.open) {
      dialog.close();
    } else {
      open();
    }
  });
}