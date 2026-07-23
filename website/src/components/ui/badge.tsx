import { cn } from "@/lib/utils";

export function Badge({
  className,
  children,
}: {
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-[3px] border border-[var(--hairline)] bg-[var(--canvas-soft)] px-2 py-0.5 font-mono text-[11px] text-[var(--body)]",
        className,
      )}
    >
      {children}
    </span>
  );
}
