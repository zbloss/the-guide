import { useState, useEffect } from 'react';
import { useParams } from 'react-router-dom';
import { useApi } from '../hooks/useApi';
import { listEncounters, createEncounter, deleteEncounter } from '../api/encounters';
import { listSessions } from '../api/sessions';
import { listCharacters } from '../api/characters';
import { EncounterList } from '../components/encounters/EncounterList';
import { EncounterForm } from '../components/encounters/EncounterForm';
import { GenerateEncounterPanel } from '../components/encounters/GenerateEncounterPanel';
import { TemplateLibrary } from '../components/encounters/TemplateLibrary';
import { Modal } from '../components/common/Modal';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import { cacheItems, getCachedItems } from '../lib/offlineDb';
import { useNetworkStatus } from '../hooks/useNetworkStatus';
import type { EncounterSummary, Session, Character, CreateEncounterRequest, EncounterTemplate } from '../api/types';

export function EncountersPage() {
  const { campaignId } = useParams<{ campaignId: string }>();
  const { isOnline } = useNetworkStatus();
  const [encounters, setEncounters] = useState<EncounterSummary[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [tick, setTick] = useState(0);

  const { data: sessions } = useApi<Session[]>(() => listSessions(campaignId!), [campaignId]);
  const { data: characters } = useApi<Character[]>(() => listCharacters(campaignId!), [campaignId]);

  const [showCreate, setShowCreate] = useState(false);
  const [showGenerate, setShowGenerate] = useState(false);
  const [showTemplates, setShowTemplates] = useState(false);
  const [templateForm, setTemplateForm] = useState<CreateEncounterRequest | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    listEncounters(campaignId!)
      .then((result) => {
        if (!cancelled) {
          setEncounters(result);
          setLoading(false);
          cacheItems('encounters', result).catch(console.error);
        }
      })
      .catch(async (err: unknown) => {
        if (!cancelled) {
          const cached = await getCachedItems<EncounterSummary>('encounters');
          if (cached.length > 0) {
            setEncounters(cached);
          } else {
            setError(err instanceof Error ? err.message : String(err));
          }
          setLoading(false);
        }
      });
    return () => { cancelled = true; };
  }, [campaignId, tick]);

  const handleCreate = async (data: CreateEncounterRequest) => {
    await createEncounter(campaignId!, data);
    setShowCreate(false);
    setTemplateForm(null);
    setTick((t) => t + 1);
  };

  const handleDelete = async (id: string) => {
    await deleteEncounter(campaignId!, id);
    setTick((t) => t + 1);
  };

  const handleLoadTemplate = (template: EncounterTemplate) => {
    setTemplateForm({
      name: template.name,
      description: template.description ?? undefined,
      participant_character_ids: [],
    });
    setShowTemplates(false);
    setShowCreate(true);
  };

  return (
    <div className="page-section">
      <div className="section-header">
        <h2>Encounters</h2>
        <div className="btn-group">
          <button
            className="btn btn-primary"
            onClick={() => setShowCreate(true)}
            disabled={!isOnline}
            title={!isOnline ? 'Unavailable offline' : undefined}
          >
            + New Encounter
          </button>
          <button
            className="btn"
            onClick={() => setShowGenerate(!showGenerate)}
            disabled={!isOnline}
            title={!isOnline ? 'Unavailable offline' : undefined}
          >
            Generate
          </button>
          <button className="btn" onClick={() => setShowTemplates(!showTemplates)}>Templates</button>
        </div>
      </div>

      {showGenerate && <GenerateEncounterPanel campaignId={campaignId!} onSaved={() => setTick((t) => t + 1)} />}

      {showTemplates && (
        <TemplateLibrary onLoad={handleLoadTemplate} />
      )}

      {loading && <LoadingSpinner />}
      {error && <ErrorBanner message={error} />}
      {encounters && <EncounterList encounters={encounters} campaignId={campaignId!} onDelete={handleDelete} />}

      {showCreate && (
        <Modal title="New Encounter" onClose={() => { setShowCreate(false); setTemplateForm(null); }}>
          <EncounterForm
            sessions={sessions ?? []}
            characters={characters ?? []}
            onSubmit={handleCreate}
            onCancel={() => { setShowCreate(false); setTemplateForm(null); }}
            initialValues={templateForm ?? undefined}
          />
        </Modal>
      )}
    </div>
  );
}
