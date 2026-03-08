import { useState } from 'react';
import { useParams } from 'react-router-dom';
import { useApi } from '../hooks/useApi';
import { listSessions, createSession, deleteSession } from '../api/sessions';
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

  const handleCreate = async (data: CreateSessionRequest) => {
    await createSession(campaignId!, data);
    setShowCreate(false);
    refetch();
  };

  const handleDelete = async (id: string) => {
    await deleteSession(campaignId!, id);
    refetch();
  };

  return (
    <div className="page-section">
      <div className="section-header">
        <h2>Sessions</h2>
        <button className="btn btn-primary" onClick={() => setShowCreate(true)}>+ New Session</button>
      </div>

      {loading && <LoadingSpinner />}
      {error && <ErrorBanner message={error} />}
      {sessions && <SessionList sessions={sessions} campaignId={campaignId!} onDelete={handleDelete} />}

      {showCreate && (
        <Modal title="New Session" onClose={() => setShowCreate(false)}>
          <SessionForm onSubmit={handleCreate} onCancel={() => setShowCreate(false)} />
        </Modal>
      )}
    </div>
  );
}
