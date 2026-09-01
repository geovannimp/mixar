import { scroll } from "motion";

export function initHeroScroll() {
  const target = document.getElementById("hero-scroll-target");
  const visual = document.getElementById("hero-visual");

  if (!target || !visual) return;
  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;

  const update = (progress: number) => {
    const desktop = window.matchMedia("(min-width: 1024px)").matches;
    const y = progress * (desktop ? -72 : -220);
    const x = desktop ? progress * -(visual.offsetWidth * 0.92) : 0;
    const scale = 1 + progress * (desktop ? 0.14 : 0.1);

    visual.style.transform = `translate3d(${x}px, ${y}px, 0) scale(${scale})`;
  };

  scroll(update, { target, offset: ["start start", "end end"] });
}
