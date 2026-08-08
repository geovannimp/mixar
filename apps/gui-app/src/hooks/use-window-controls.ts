import { useCallback, useEffect, useRef, useState } from "react";
import { getAppWindow } from "@/lib/tauri-app";

export const useWindowControls = () => {
  const appWindow = useRef(getAppWindow());
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    let disposed = false;

    const syncMaximized = async () => {
      const maximized = await appWindow.current.isMaximized();
      if (!disposed) {
        setIsMaximized(maximized);
      }
    };

    void syncMaximized();

    const unlistenPromise = appWindow.current.onResized(() => {
      void syncMaximized();
    });

    return () => {
      disposed = true;
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  const toggleMaximize = useCallback(() => {
    void appWindow.current.toggleMaximize();
  }, []);

  const minimize = useCallback(() => {
    void appWindow.current.minimize();
  }, []);

  const close = useCallback(() => {
    void appWindow.current.close();
  }, []);

  return {
    isMaximized,
    toggleMaximize,
    minimize,
    close,
  };
};
