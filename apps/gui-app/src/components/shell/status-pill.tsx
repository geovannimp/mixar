import { statusPillClass } from "@/lib/ui";
import { cn } from "@/lib/utils";

interface StatusPillProps {
  active: boolean;
  children: React.ReactNode;
  className?: string;
}

export function StatusPill({ active, children, className }: StatusPillProps) {
  return <span className={cn(statusPillClass(active), className)}>{children}</span>;
}
