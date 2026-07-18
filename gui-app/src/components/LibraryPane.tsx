import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

interface LibraryPaneProps {
  title: string;
  titleTooltip?: string;
  tabs?: ReactNode;
  headerInline?: ReactNode;
  headerAction?: ReactNode;
  scrollable?: boolean;
  children: ReactNode;
}

export function LibraryPane({
  title,
  titleTooltip,
  tabs,
  headerInline,
  headerAction,
  scrollable = true,
  children,
}: LibraryPaneProps) {
  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="shrink-0 space-y-2 px-3 py-2">
        {tabs}
        <LibraryPaneHeader
          title={title}
          titleTooltip={titleTooltip}
          inline={headerInline}
          action={headerAction}
        />
      </div>
      <div
        className={cn(
          "min-h-0 flex-1 px-2 pb-3",
          scrollable ? "overflow-y-auto" : "flex flex-col overflow-hidden",
        )}
      >
        {children}
      </div>
    </div>
  );
}

interface LibraryPaneHeaderProps {
  title: string;
  titleTooltip?: string;
  inline?: ReactNode;
  action?: ReactNode;
}

function LibraryPaneHeader({ title, titleTooltip, inline, action }: LibraryPaneHeaderProps) {
  return (
    <div
      className={
        inline ? "flex min-h-8 items-center gap-2" : "flex h-6 items-center justify-between gap-2"
      }
    >
      <p
        className="shrink-0 text-[10px] font-semibold uppercase tracking-widest text-zinc-600"
        title={titleTooltip ?? title}
      >
        {title}
      </p>
      {inline && <div className="min-w-0 flex-1">{inline}</div>}
      {action && (
        <div
          className={
            inline
              ? "flex shrink-0 items-center justify-center"
              : "flex h-6 w-6 shrink-0 items-center justify-center"
          }
        >
          {action}
        </div>
      )}
    </div>
  );
}
