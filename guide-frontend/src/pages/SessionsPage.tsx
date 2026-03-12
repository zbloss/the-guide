import { useState } from 'react';
import { useParams } from 'react-router-dom';
import { useApi } from '../hooks/useApi';
import { listSessions, createSession, deleteSession, generateSessionPrep } from '../api/sessions';
import { SessionList } from '../components/sessions/SessionList';
import { SessionForm } from '../components/sessions/SessionForm';
import { Modal } from '../components/common/Modal';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import type { Session, CreateSessionRequest } from '../api/types';

export function SessionsPage() {
  const { campaignId } = useParams<{ campaignId: string }>();
  const { data: sessions, loading, error, refetch } = useApi<Session[]>(
    () => listSessions(campaignId!),
    [campaignId],
  );
  const [showCreate, setShowCreate] = useState(false);

  const [showPrep, setShowPrep] = useState(false);
  const [prepNotes, setPrepNotes] = useState('');
  const [prepResult, setPrepResult] = useState('');
  const [prepLoading, setPrepLoading] = useState(false);
  const [prepError, setPrepError] = useState('');
  const [prepCopied, setPrepCopied] = useState(false);

  const handleCreate = async (data: CreateSessionRequest) => {
    await createSession(campaignId!, data);
    setShowCreate(false);
    refetch();
  };

  const handleDelete = async (id: string) => {
    await deleteSession(campaignId!, id);
    refetch();
  };

  const handleGeneratePrep = async () => {
    setPrepLoading(true);
    setPrepError('');
    setPrepResult('');
    setPrepCopied(false);
    try {
      const data = await generateSessionPrep(campaignId!, prepNotes || undefined);
      setPrepResult(data.prep);
    } catch (e: unknown) {
      setPrepError(e instanceof Error ? e.message : String(e));
    } finally {
      setPrepLoading(false);
    }
  };

  const handlePrepClose = () => {
    setShowPrep(false);
    setPrepNotes('');
    setPrepResult('');
    setPrepError('');
    setPrepCopied(false);
  };

  const handlePrepCopy = () => {
    navigator.clipboard.writeText(prepResult).then(() => {
      setPrepCopied(true);
      setTimeout(() => setPrepCopied(false), 2000);
    });
  };

  return (
    <div className="page-section">
      <div className="section-header">
        <h2>Sessions</h2>
        <div style={{ display: 'flex', gap: '0.5rem' }}>
          <button className="btn btn-sm btn-primary" onClick={() => setShowPrep(true)}>Session Prep</button>
          <button className="btn btn-primary" onClick={() => setShowCreate(true)}>+ New Session</button>
        </div>
      </div>

      {loading && <LoadingSpinner />}
      {error && <ErrorBanner message={error} />}
      {sessions && <SessionList sessions={sessions} campaignId={campaignId!} onDelete={handleDelete} />}

      {showCreate && (
        <Modal title="New Session" onClose={() => setShowCreate(false)}>
          <SessionForm onSubmit={handleCreate} onCancel={() => setShowCreate(false)} />
        </Modal>
      )}

      {showPrep && (
        <Modal title="Session Prep Assistant" onClose={handlePrepClose}>
          <div className="session-prep-modal">
            <div className="form-field">
              <label className="form-label">Upcoming Notes (optional)</label>
              <textarea
                className="form-input"
                rows={3}
                placeholder="Add any notes about what you plan for the next session..."
                value={prepNotes}
                onChange={(e) => setPrepNotes(e.target.value)}
                disabled={prepLoading}
              />
            </div>

            <div className="form-actions">
              <button
                className="btn btn-primary"
                onClick={handleGeneratePrep}
                disabled={prepLoading}
              >
                {prepLoading ? <><LoadingSpinner size={14} /> Generating...</> : 'Generate Prep'}
              </button>
              <button className="btn" onClick={handlePrepClose} disabled={prepLoading}>
                Cancel
              </button>
            </div>

            {prepError && <div className="form-error-banner">{prepError}</div>}

            {prepResult && (
              <div className="session-prep-result">
                <div className="session-prep-output" style={{ maxHeight: '400px', overflowY: 'auto', whiteSpace: 'pre-wrap', fontFamily: 'inherit', padding: '1rem', background: 'var(--color-surface, #1e1e2e)', borderRadius: '6px', marginTop: '1rem', fontSize: '0.9rem', lineHeight: '1.6' }}>
                  {prepResult}
                </div>
                <div className="form-actions" style={{ marginTop: '0.5rem' }}>
                  <button className="btn btn-sm" onClick={handlePrepCopy}>
                    {prepCopied ? 'Copied!' : 'Copy to Clipboard'}
                  </button>
                </div>
              </div>
            )}
          </div>
        </Modal>
      )}
    </div>
  );
}
