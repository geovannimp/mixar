const START_ROTATE_Y = -10;
const START_ROTATE_X = 12;
const SCROLL_DISTANCE = 360;

export function initHeroMobileTilt() {
  const tilt = document.querySelector<HTMLElement>(".hero-screenshot-tilt");
  if (!tilt) return;

  const isDesktop = () => window.matchMedia("(min-width: 1024px)").matches;
  const prefersReducedMotion = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  let ticking = false;

  const update = () => {
    ticking = false;

    if (isDesktop() || prefersReducedMotion()) {
      tilt.style.removeProperty("transform");
      return;
    }

    const progress = Math.min(1, Math.max(0, window.scrollY / SCROLL_DISTANCE));
    const rotateY = START_ROTATE_Y * (1 - progress);
    const rotateX = START_ROTATE_X * (1 - progress);
    tilt.style.transform = `rotateY(${rotateY}deg) rotateX(${rotateX}deg)`;
  };

  const onScroll = () => {
    if (ticking) return;
    ticking = true;
    requestAnimationFrame(update);
  };

  window.addEventListener("scroll", onScroll, { passive: true });
  window.addEventListener("resize", onScroll, { passive: true });
  update();
}
