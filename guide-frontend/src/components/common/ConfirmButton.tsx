import { useState, useRef, useEffect } from 'react';

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
  const spanRef = useRef<HTMLSpanElement>(null);

  useEffect(() => {
    if (!confirming) return;
    const handleMouseDown = (e: MouseEvent) => {
      if (spanRef.current && !spanRef.current.contains(e.target as Node)) {
        setConfirming(false);
      }
    };
    document.addEventListener('mousedown', handleMouseDown);
    return () => document.removeEventListener('mousedown', handleMouseDown);
  }, [confirming]);

  if (confirming) {
    return (
      <span className="confirm-inline" ref={spanRef}>
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
