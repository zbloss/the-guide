import { useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import { useApi } from '../hooks/useApi';
import { getSession, startSession, endSession, listEvents, createEvent, getSessionSummary } from '../api/sessions';
import { listCharacters } from '../api/characters';
import { SessionEventList } from '../components/sessions/SessionEventList';
import { SessionEventForm } from '../components/sessions/SessionEventForm';
import { SummaryView } from '../components/sessions/SummaryView';
import { PerspectiveSelector } from '../components/chat/PerspectiveSelector';
import { Modal } from '../components/common/Modal';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import type { Session, SessionEvent, SessionSummary, Perspective, Character, CreateSessionEventRequest } from '../api/types';

export function SessionDetailPage() {
  const { campaignId, sessionId } = useParams<{ campaignId: string; sessionId: string }>();
  const { data: session, loading, error, refetch: refetchSession } = useApi<Session>(
    () => getSession(campaignId!, sessionId!),
    [campaignId, sessionId],
  );
  const { data: events, refetch: refetchEvents } = useApi<SessionEvent[]>(
    () => listEvents(campaignId!, sessionId!),
    [campaignId, sessionId],
  );
  const { data: characters } = useApi<Character[]>(
    () => listCharacters(campaignId!),
    [campaignId],
  );

  const [tab, setTab] = useState<'events' | 'summary'>('events');
  const [showAddEvent, setShowAddEvent] = useState(false);
  const [perspective, setPerspective] = useState<Perspective>('dm');
  const [summary, setSummary] = useState<SessionSummary | null>(null);
  const [summaryLoading, setSummaryLoading] = useState(false);
  const [summaryError, setSummaryError] = useState('');
  const [actionError, setActionError] = useState('');

  const handleStart = async () => {
    setActionError('');
    try {
      await startSession(campaignId!, sessionId!);
      refetchSession();
    } catch (e: unknown) {
      setActionError(e instanceof Error ? e.message : String(e));
    }
  };

  const handleEnd = async () => {
    setActionError('');
    try {
      await endSession(campaignId!, sessionId!);
      refetchSession();
    } catch (e: unknown) {
      setActionError(e instanceof Error ? e.message : String(e));
    }
  };

  const handleAddEvent = async (data: CreateSessionEventRequest) => {
    await createEvent(campaignId!, sessionId!, data);
    setShowAddEvent(false);
    refetchEvents();
  };

  const handleGenerateSummary = async () => {
    setSummaryLoading(true);
    setSummaryError('');
    setSummary(null);
    try {
      const result = await getSessionSummary(campaignId!, sessionId!, perspective);
      setSummary(result);
    } catch (e: unknown) {
      setSummaryError(e instanceof Error ? e.message : String(e));
    } finally {
      setSummaryLoading(false);
    }
  };

  if (loading) return <div className="page"><LoadingSpinner /></div>;
  if (error) return <div className="page"><ErrorBanner message={error} /></div>;
  if (!session) return null;

  return (
    <div className="page">
      <div className="page-header">
        <div>
          <Link to={`/campaigns/${campaignId}/sessions`} className="breadcrumb">← Sessions</Link>
          <h1>Session {session.session_number}{session.title ? `: ${session.title}` : ''}</h1>
        </div>
        <div className="session-actions">
          {session.status === 'pending' && (
            <button className="btn btn-primary" onClick={handleStart}>Start Session</button>
          )}
          {session.status === 'started' && (
            <button className="btn btn-danger" onClick={handleEnd}>End Session</button>
          )}
          {session.status === 'ended' && <span className="badge badge-default">Ended</span>}
        </div>
      </div>

      {actionError && <ErrorBanner message={actionError} />}

      <div className="tab-nav">
        <button className={`tab-link ${tab === 'events' ? 'active' : ''}`} onClick={() => setTab('events')}>Events</button>
        <button className={`tab-link ${tab === 'summary' ? 'active' : ''}`} onClick={() => setTab('summary')}>Summary</button>
      </div>

      {tab === 'events' && (
        <div>
          <div className="section-header">
            <h3>Events</h3>
            <button className="btn btn-sm btn-primary" onClick={() => setShowAddEvent(true)}>+ Add Event</button>
          </div>
          {events && <SessionEventList events={events} />}

          {showAddEvent && (
            <Modal title="Add Event" onClose={() => setShowAddEvent(false)}>
              <SessionEventForm
                characters={characters ?? []}
                onSubmit={handleAddEvent}
                onCancel={() => setShowAddEvent(false)}
              />
            </Modal>
          )}
        </div>
      )}

      {tab === 'summary' && (
        <div className="summary-tab">
          <div className="summary-controls">
            <PerspectiveSelector value={perspective} onChange={setPerspective} disabled={summaryLoading} />
            <button className="btn btn-primary" onClick={handleGenerateSummary} disabled={summaryLoading}>
              {summaryLoading ? <><LoadingSpinner size={14} /> Generating…</> : 'Generate Summary'}
            </button>
          </div>
          {summaryError && <ErrorBanner message={summaryError} />}
          {summary && <SummaryView summary={summary} />}
          {!summary && !summaryLoading && !summaryError && (
            <p className="empty-state">Click "Generate Summary" to create an AI-powered session recap.</p>
          )}
        </div>
      )}
    </div>
  );
}
