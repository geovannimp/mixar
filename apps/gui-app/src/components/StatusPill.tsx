import { statusPillClass } from "@/lib/ui";

interface StatusPillProps {
  active: boolean;
  children: React.ReactNode;
}

export function StatusPill({ active, children }: StatusPillProps) {
  return <span className={statusPillClass(active)}>{children}</span>;
}
