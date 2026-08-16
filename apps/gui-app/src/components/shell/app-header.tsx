import mixarLogo from "@/assets/mixar-logo.png";
import { isTauriApp } from "@/lib/tauri-app";

import { TitleBarDragRegion } from "./title-bar-drag-region";
import { WindowTitleBarControls } from "./window-title-bar-controls";
import { AppHeaderTabs } from "@/components/shell/app-header-tabs";
import { AppHeaderMasterControls } from "@/components/shell/app-header-master-controls";

export function AppHeader() {
  const showWindowControls = isTauriApp();

  return (
    <header className="relative z-40 flex h-10 shrink-0 items-stretch border-b border-white/8">
      <div className="flex min-w-0 items-center gap-3 px-4">
        <h1 className="flex items-center">
          <img src={mixarLogo} alt="Mixar" className="h-4 w-auto invert" />
        </h1>
      </div>

      <TitleBarDragRegion className="min-w-6 flex-1" />

      <div className="absolute inset-x-0 top-0 bottom-0 z-10 flex items-center justify-center pointer-events-none">
        <nav
          className="pointer-events-auto font-serif"
          onMouseDown={(event) => {
            event.stopPropagation();
          }}
        >
          <AppHeaderTabs />
        </nav>
      </div>

      <AppHeaderMasterControls />

      {showWindowControls && <WindowTitleBarControls />}
    </header>
  );
}
