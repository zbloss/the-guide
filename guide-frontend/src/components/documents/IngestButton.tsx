import { useState, useEffect, useRef, useCallback } from 'react';
import { LoadingSpinner } from '../common/LoadingSpinner';
import type { IngestionStatus } from '../../api/types';

interface IngestButtonProps {
  docId: string;
  currentStatus: IngestionStatus;
  onIngest: () => Promise<void>;
  onPoll: () => Promise<{ ingestion_status: IngestionStatus }>;
  onComplete?: () => void;
}

export function IngestButton({ docId, currentStatus, onIngest, onPoll, onComplete }: IngestButtonProps) {
  const [status, setStatus] = useState<IngestionStatus>(currentStatus);
  const [error, setError] = useState('');
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const processingStartRef = useRef<number | null>(currentStatus === 'processing' ? Date.now() : null);

  // Sync with prop changes
  useEffect(() => {
    setStatus(currentStatus);
  }, [currentStatus]);

  const clearPoller = useCallback(() => {
    if (intervalRef.current) {
      clearInterval(intervalRef.current);
      intervalRef.current = null;
    }
  }, []);

  useEffect(() => {
    return () => clearPoller();
  }, [docId, clearPoller]);

  const startPolling = useCallback(() => {
    clearPoller();
    intervalRef.current = setInterval(async () => {
      try {
        const doc = await onPoll();
        setStatus(doc.ingestion_status);
        if (doc.ingestion_status === 'completed') {
          clearPoller();
          onComplete?.();
        } else if (doc.ingestion_status === 'failed') {
          clearPoller();
        }
      } catch {
        clearPoller();
      }
    }, 3000);
  }, [clearPoller, onPoll, onComplete]);

  // Auto-start polling if doc was already processing on mount
  useEffect(() => {
    if (currentStatus === 'processing') {
      startPolling();
    }
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const handleIngest = async () => {
    setError('');
    processingStartRef.current = Date.now();
    try {
      await onIngest();
      setStatus('processing');
      startPolling();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  if (status === 'processing') {
    const elapsedSec = processingStartRef.current ? Math.floor((Date.now() - processingStartRef.current) / 1000) : 0;
    return (
      <>
        <span className="ingest-status"><LoadingSpinner size={14} /> Processing…</span>
        {elapsedSec > 60 && (
          <button className="btn btn-sm" onClick={handleIngest} style={{ marginLeft: 8 }}>Force Retry</button>
        )}
      </>
    );
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
