import type { TrackDropData } from "@/lib/track-drag";
import { FALLBACK_AUDIO_EXTENSIONS } from "@/lib/audio-extensions";

export type OsFileDropTarget = {
  id: string;
  element: HTMLElement;
  data: TrackDropData;
  collisionPriority: number;
  disabled: boolean;
  setHover: (hover: boolean) => void;
};

const targets = new Map<string, OsFileDropTarget>();

export function registerOsFileDropTarget(target: OsFileDropTarget): () => void {
  targets.set(target.id, target);
  return () => {
    const current = targets.get(target.id);
    if (current === target) {
      current.setHover(false);
      targets.delete(target.id);
    }
  };
}

export function updateOsFileDropTarget(
  id: string,
  patch: Partial<Pick<OsFileDropTarget, "disabled" | "collisionPriority" | "data">>,
): void {
  const current = targets.get(id);
  if (!current) {
    return;
  }
  Object.assign(current, patch);
}

/** Prefer the deepest registered zone under the point; break ties by collisionPriority. */
export function findOsFileDropTargetAt(clientX: number, clientY: number): OsFileDropTarget | null {
  const el = document.elementFromPoint(clientX, clientY);
  if (!el) {
    return null;
  }

  const candidates: OsFileDropTarget[] = [];
  let node: Element | null = el;
  while (node) {
    if (node instanceof HTMLElement) {
      const id = node.dataset.trackDropZone;
      if (id) {
        const target = targets.get(id);
        if (target && !target.disabled && target.element === node) {
          candidates.push(target);
        }
      }
    }
    node = node.parentElement;
  }

  if (candidates.length === 0) {
    return null;
  }

  return candidates.reduce((best, next) =>
    next.collisionPriority >= best.collisionPriority ? next : best,
  );
}

export function clearOsFileDropHover(): void {
  for (const target of targets.values()) {
    target.setHover(false);
  }
}

export function setOsFileDropHover(target: OsFileDropTarget | null): void {
  for (const entry of targets.values()) {
    entry.setHover(entry === target);
  }
}

const AUDIO_EXT = new Set(FALLBACK_AUDIO_EXTENSIONS.map((ext) => ext.toLowerCase()));

export function filterAudioFilePaths(paths: string[]): string[] {
  return paths.filter((path) => {
    const base = path.split(/[/\\]/).pop() ?? path;
    const dot = base.lastIndexOf(".");
    if (dot < 0) {
      return false;
    }
    return AUDIO_EXT.has(base.slice(dot + 1).toLowerCase());
  });
}
