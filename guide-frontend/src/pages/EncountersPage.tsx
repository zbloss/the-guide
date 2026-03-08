import { useState } from 'react';
import { useParams } from 'react-router-dom';
import { useApi } from '../hooks/useApi';
import { listEncounters, createEncounter, deleteEncounter } from '../api/encounters';
import { listSessions } from '../api/sessions';
import { listCharacters } from '../api/characters';
import { EncounterList } from '../components/encounters/EncounterList';
import { EncounterForm } from '../components/encounters/EncounterForm';
import { GenerateEncounterPanel } from '../components/encounters/GenerateEncounterPanel';
import { Modal } from '../components/common/Modal';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import type { EncounterSummary, Session, Character, CreateEncounterRequest } from '../api/types';

export function EncountersPage() {
  const { campaignId } = useParams<{ campaignId: string }>();
  const { data: encounters, loading, error, refetch } = useApi<EncounterSummary[]>(
    () => listEncounters(campaignId!),
    [campaignId],
  );
  const { data: sessions } = useApi<Session[]>(() => listSessions(campaignId!), [campaignId]);
  const { data: characters } = useApi<Character[]>(() => listCharacters(campaignId!), [campaignId]);

  const [showCreate, setShowCreate] = useState(false);
  const [showGenerate, setShowGenerate] = useState(false);

  const handleCreate = async (data: CreateEncounterRequest) => {
    await createEncounter(campaignId!, data);
    setShowCreate(false);
    refetch();
  };

  const handleDelete = async (id: string) => {
    await deleteEncounter(campaignId!, id);
    refetch();
  };

  return (
    <div className="page-section">
      <div className="section-header">
        <h2>Encounters</h2>
        <div className="btn-group">
          <button className="btn btn-primary" onClick={() => setShowCreate(true)}>+ New Encounter</button>
          <button className="btn" onClick={() => setShowGenerate(!showGenerate)}>🎲 Generate</button>
        </div>
      </div>

      {showGenerate && <GenerateEncounterPanel campaignId={campaignId!} />}

      {loading && <LoadingSpinner />}
      {error && <ErrorBanner message={error} />}
      {encounters && <EncounterList encounters={encounters} campaignId={campaignId!} onDelete={handleDelete} />}

      {showCreate && (
        <Modal title="New Encounter" onClose={() => setShowCreate(false)}>
          <EncounterForm
            sessions={sessions ?? []}
            characters={characters ?? []}
            onSubmit={handleCreate}
            onCancel={() => setShowCreate(false)}
          />
        </Modal>
      )}
    </div>
  );
}
