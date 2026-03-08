import { useState, useEffect, useRef } from 'react';
import { LoadingSpinner } from '../common/LoadingSpinner';
import type { IngestionStatus } from '../../api/types';

interface IngestButtonProps {
  docId: string;
  currentStatus: IngestionStatus;
  onIngest: () => Promise<void>;
  onPoll: () => Promise<{ status: IngestionStatus }>;
}

export function IngestButton({ docId, currentStatus, onIngest, onPoll }: IngestButtonProps) {
  const [status, setStatus] = useState<IngestionStatus>(currentStatus);
  const [error, setError] = useState('');
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Sync with prop changes
  useEffect(() => {
    setStatus(currentStatus);
  }, [currentStatus]);

  const clearPoller = () => {
    if (intervalRef.current) {
      clearInterval(intervalRef.current);
      intervalRef.current = null;
    }
  };

  useEffect(() => {
    return () => clearPoller();
  }, [docId]);

  const startPolling = () => {
    clearPoller();
    intervalRef.current = setInterval(async () => {
      try {
        const doc = await onPoll();
        setStatus(doc.status);
        if (doc.status === 'completed' || doc.status === 'failed') {
          clearPoller();
        }
      } catch {
        clearPoller();
      }
    }, 3000);
  };

  const handleIngest = async () => {
    setError('');
    try {
      await onIngest();
      setStatus('processing');
      startPolling();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  if (status === 'processing') {
    return <span className="ingest-status"><LoadingSpinner size={14} /> Processing…</span>;
  }
  if (status === 'completed') {
    return <span className="badge badge-success">Ingested ✓</span>;
  }
  if (status === 'failed') {
    return (
      <>
        <span className="badge badge-danger">Failed</span>
        <button className="btn btn-sm" onClick={handleIngest}>Retry</button>
        {error && <span className="form-error">{error}</span>}
      </>
    );
  }

  return (
    <>
      <button className="btn btn-sm btn-primary" onClick={handleIngest}>Ingest</button>
      {error && <span className="form-error">{error}</span>}
    </>
  );
}
