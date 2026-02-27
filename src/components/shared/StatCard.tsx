export function StatCard({
  label,
  value,
  icon,
  truncate,
}: {
  label: string;
  value: string;
  icon?: React.ReactNode;
  truncate?: boolean;
}) {
  return (
    <div className="rounded-xl border border-border bg-card p-3 text-center">
      <div className="flex items-center justify-center gap-1">
        {icon}
        <p className="text-xs text-muted-foreground">{label}</p>
      </div>
      <p
        className={`mt-1 text-sm font-semibold text-foreground ${truncate ? "truncate" : ""}`}
        title={truncate ? value : undefined}
      >
        {value}
      </p>
    </div>
  );
}
