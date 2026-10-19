/**
 * Triggers the ghost outline word's stroke-dash draw once it scrolls into
 * view. Shared by the landing Features grid and the developers Stack grid so
 * the draw threshold and class name stay in one place. Under reduced motion
 * the CSS end state is already drawn, so adding the class is a harmless no-op.
 */
export function drawGhostWordOnEnter(selector = ".ghost-word"): void {
  const word = document.querySelector(selector);
  if (!word || word.classList.contains("is-drawn")) return;

  const observer = new IntersectionObserver(
    ([entry]) => {
      if (!entry?.isIntersecting) return;
      word.classList.add("is-drawn");
      observer.disconnect();
    },
    { threshold: 0.3 },
  );
  observer.observe(word);
}