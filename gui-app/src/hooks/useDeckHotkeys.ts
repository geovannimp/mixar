import { useEffect } from "react";

interface UseDeckHotkeysOptions {
  focusedDeckId: number;
  onTriggerHotCue: (deckId: number, slot: number) => void;
}

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  const tag = target.tagName;
  return (
    target.isContentEditable ||
    tag === "INPUT" ||
    tag === "TEXTAREA" ||
    tag === "SELECT"
  );
}

export function useDeckHotkeys({
  focusedDeckId,
  onTriggerHotCue,
}: UseDeckHotkeysOptions): void {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || isEditableTarget(event.target)) {
        return;
      }
      if (event.ctrlKey || event.metaKey || event.altKey) {
        return;
      }

      const slot = Number.parseInt(event.key, 10);
      if (!Number.isFinite(slot) || slot < 1 || slot > 8) {
        return;
      }

      event.preventDefault();
      onTriggerHotCue(focusedDeckId, slot - 1);
    };

    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [focusedDeckId, onTriggerHotCue]);
}
