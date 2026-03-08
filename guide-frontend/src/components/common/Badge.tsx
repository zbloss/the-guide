interface BadgeProps {
  label: string;
  variant?: 'default' | 'success' | 'warning' | 'danger' | 'info' | 'pulse';
}

export function Badge({ label, variant = 'default' }: BadgeProps) {
  return <span className={`badge badge-${variant}`}>{label}</span>;
}

// Specific badge for IngestionStatus
export function StatusBadge({ status }: { status: string }) {
  const variant =
    status === 'completed' ? 'success' :
    status === 'failed' ? 'danger' :
    status === 'processing' ? 'pulse' :
    'default';
  return <Badge label={status} variant={variant} />;
}
