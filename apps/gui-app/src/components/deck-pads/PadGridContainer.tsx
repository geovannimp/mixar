import type { ReactNode } from "react";

interface PadGridContainerProps {
  children: ReactNode;
}

export function PadGridContainer({ children }: PadGridContainerProps) {
  return (
    <div className="grid min-h-0 flex-1 grid-cols-4 gap-1.5 p-2 sm:gap-2 sm:p-2.5">{children}</div>
  );
}
