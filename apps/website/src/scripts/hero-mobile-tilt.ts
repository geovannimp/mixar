import { animate, scroll, type AnimationPlaybackControls } from "motion";

const MOBILE_ROTATE_Y = 0;
const MOBILE_ROTATE_X = 24;
const MOBILE_SCALE = 1.1;
const SCROLL_DISTANCE = 256;

const DESKTOP_ROTATE_Y = -7;
const DESKTOP_ROTATE_X = 8;
const FLOAT_DURATION = 20;

export function initHeroMobileTilt() {
  const tilt = document.querySelector<HTMLElement>(".hero-screenshot-tilt");
  if (!tilt) return;

  const desktop = window.matchMedia("(min-width: 1024px)");

  let cancelScroll: (() => void) | undefined;
  let floatAnimation: AnimationPlaybackControls | undefined;
  let unbindHover: (() => void) | undefined;

  const clearTransforms = () => {
    floatAnimation?.stop();
    floatAnimation = undefined;
    tilt.style.removeProperty("transform");
  };

  const mountMobile = () => {
    const animation = animate(
      tilt,
      {
        rotateY: [MOBILE_ROTATE_Y, 0],
        rotateX: [MOBILE_ROTATE_X, 0],
        scale: [MOBILE_SCALE, 1],
      },
      { ease: "easeInOut", autoplay: false },
    );

    cancelScroll = scroll(
      (progress: number) => {
        animation.time = progress * animation.duration;
      },
      { offset: ["0px start", `${SCROLL_DISTANCE}px start`] },
    );
  };

  const mountDesktop = () => {
    const straight = { rotateY: 0, rotateX: 0, x: 0, y: 0 };

    const startFloat = () => {
      floatAnimation?.stop();
      floatAnimation = animate(
        tilt,
        {
          rotateY: DESKTOP_ROTATE_Y,
          rotateX: DESKTOP_ROTATE_X,
          x: [0, 6],
          y: [0, -10],
        },
        {
          duration: FLOAT_DURATION / 2,
          ease: "easeInOut",
          repeat: Infinity,
          repeatType: "reverse",
        },
      );
    };

    const onEnter = () => {
      floatAnimation?.stop();
      animate(tilt, straight, { duration: 0.35, ease: "easeOut" });
    };

    const onLeave = () => {
      startFloat();
    };

    tilt.addEventListener("mouseenter", onEnter);
    tilt.addEventListener("mouseleave", onLeave);
    unbindHover = () => {
      tilt.removeEventListener("mouseenter", onEnter);
      tilt.removeEventListener("mouseleave", onLeave);
    };

    startFloat();
  };

  const mount = () => {
    cancelScroll?.();
    cancelScroll = undefined;
    unbindHover?.();
    unbindHover = undefined;
    clearTransforms();

    if (desktop.matches) {
      mountDesktop();
    } else {
      mountMobile();
    }
  };

  desktop.addEventListener("change", mount);
  mount();
}
