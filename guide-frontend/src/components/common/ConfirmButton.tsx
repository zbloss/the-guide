import { useState } from 'react';

interface ConfirmButtonProps {
  label: string;
  confirmLabel?: string;
  onConfirm: () => void;
  variant?: 'danger' | 'default';
  disabled?: boolean;
}

export function ConfirmButton({
  label,
  confirmLabel = 'Are you sure?',
  onConfirm,
  variant = 'default',
  disabled = false,
}: ConfirmButtonProps) {
  const [confirming, setConfirming] = useState(false);

  if (confirming) {
    return (
      <span className="confirm-inline">
        <span className="confirm-label">{confirmLabel}</span>
        <button
          className="btn btn-danger btn-sm"
          onClick={() => { setConfirming(false); onConfirm(); }}
        >
          Yes
        </button>
        <button className="btn btn-sm" onClick={() => setConfirming(false)}>
          No
        </button>
      </span>
    );
  }

  return (
    <button
      className={`btn btn-sm ${variant === 'danger' ? 'btn-danger' : ''}`}
      onClick={() => setConfirming(true)}
      disabled={disabled}
    >
      {label}
    </button>
  );
}
