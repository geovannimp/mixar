import { createDocsFuseSearch, type SearchScope } from "../lib/docs-fuse-search";
import type { SearchDocument, SearchResult } from "../lib/docs-search";

type Request =
  | { type: "index"; documents: SearchDocument[] }
  | { type: "search"; id: number; query: string; scope: SearchScope };

type Response =
  | { type: "ready" }
  | { type: "results"; id: number; results: SearchResult[] }
  | { type: "error"; id?: number; message: string };

const scope = self as unknown as {
  addEventListener(type: "message", callback: (event: MessageEvent<Request>) => void): void;
  postMessage(message: Response): void;
};

let search: ReturnType<typeof createDocsFuseSearch> | null = null;

scope.addEventListener("message", ({ data }) => {
  if (data.type === "index") {
    try {
      search = createDocsFuseSearch(data.documents);
      scope.postMessage({ type: "ready" });
    } catch (error) {
      scope.postMessage({ type: "error", message: String(error) });
    }
    return;
  }

  try {
    if (search === null) throw new Error("Search index is not ready");
    scope.postMessage({
      type: "results",
      id: data.id,
      results: search(data.query, data.scope),
    });
  } catch (error) {
    scope.postMessage({ type: "error", id: data.id, message: String(error) });
  }
});
