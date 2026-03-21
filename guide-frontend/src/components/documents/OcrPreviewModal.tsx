import { useEffect, useState } from 'react';
import { Modal } from '../common/Modal';
import { LoadingSpinner } from '../common/LoadingSpinner';
import { ErrorBanner } from '../common/ErrorBanner';
import { getCampaignDocPages } from '../../api/documents';
import type { DocumentPageOcr } from '../../api/types';

interface OcrPreviewModalProps {
  campaignId: string;
  docId: string;
  filename: string;
  onClose: () => void;
}

export function OcrPreviewModal({ campaignId, docId, filename, onClose }: OcrPreviewModalProps) {
  const [pages, setPages] = useState<DocumentPageOcr[]>([]);
  const [currentPage, setCurrentPage] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getCampaignDocPages(campaignId, docId)
      .then((data) => {
        setPages(data);
        setLoading(false);
      })
      .catch((err) => {
        setError(err?.message ?? 'Failed to load OCR data');
        setLoading(false);
      });
  }, [campaignId, docId]);

  const page = pages[currentPage];

  return (
    <Modal title={`OCR Preview — ${filename}`} onClose={onClose}>
      {loading && <LoadingSpinner />}
      {error && <ErrorBanner message={error} />}

      {!loading && !error && pages.length === 0 && (
        <p className="text-muted">No OCR data available for this document.</p>
      )}

      {!loading && !error && pages.length > 0 && page && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <span style={{ fontWeight: 600 }}>Page {page.page_num} of {pages.length}</span>
            {page.is_dm_only && (
              <span className="badge badge-warning" style={{ fontSize: '0.75rem' }}>DM ONLY</span>
            )}
          </div>

          <pre style={{
            maxHeight: '60vh',
            overflowY: 'auto',
            whiteSpace: 'pre-wrap',
            wordBreak: 'break-word',
            fontFamily: 'monospace',
            fontSize: '0.85rem',
            background: 'var(--bg-secondary, #f4f4f4)',
            padding: '1rem',
            borderRadius: '4px',
            margin: 0,
          }}>
            {page.raw_text || <em style={{ opacity: 0.5 }}>(empty page)</em>}
          </pre>

          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <button
              className="btn btn-sm"
              onClick={() => setCurrentPage((p) => p - 1)}
              disabled={currentPage === 0}
            >
              ← Prev
            </button>
            <span style={{ fontSize: '0.9rem', color: 'var(--text-muted, #666)' }}>
              Page {currentPage + 1} / {pages.length}
            </span>
            <button
              className="btn btn-sm"
              onClick={() => setCurrentPage((p) => p + 1)}
              disabled={currentPage === pages.length - 1}
            >
              Next →
            </button>
          </div>
        </div>
      )}
    </Modal>
  );
}
