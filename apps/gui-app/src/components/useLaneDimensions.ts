import { useEffect, useRef, useState } from "react";

export function useLaneDimensions() {
  const ref = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });

  useEffect(() => {
    const node = ref.current;
    if (!node) {
      return;
    }

    const update = () => {
      setSize({
        width: Math.floor(node.clientWidth),
        height: Math.floor(node.clientHeight),
      });
    };

    const observer = new ResizeObserver(update);
    observer.observe(node);
    update();

    return () => observer.disconnect();
  }, []);

  return { ref, size };
}
