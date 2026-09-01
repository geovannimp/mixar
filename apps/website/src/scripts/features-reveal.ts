export function initRevealGroups() {
  const groups = document.querySelectorAll<HTMLElement>("[data-reveal-group]:not([data-reveal-init])");

  for (const group of groups) {
    group.dataset.revealInit = "";

    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      group.classList.add("is-visible");
      continue;
    }

    const observer = new IntersectionObserver(
      ([entry]) => {
        if (!entry?.isIntersecting) return;
        group.classList.add("is-visible");
        observer.disconnect();
      },
      { threshold: 0.1 },
    );

    observer.observe(group);
  }
}
