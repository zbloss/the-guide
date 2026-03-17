import { useState, useEffect, useRef } from 'react';
import { useParams, Link } from 'react-router-dom';
import { useApi } from '../hooks/useApi';
import { getSession, startSession, endSession, listEvents, createEvent, getSessionSummary, deleteEvent, generateDebrief } from '../api/sessions';
import { uploadSessionMap } from '../api/sessions';
import { listCharacters } from '../api/characters';
import { SessionEventList } from '../components/sessions/SessionEventList';
import { SessionEventForm } from '../components/sessions/SessionEventForm';
import { SummaryView } from '../components/sessions/SummaryView';
import { LootLog } from '../components/sessions/LootLog';
import { VoiceNoteCapture } from '../components/sessions/VoiceNoteCapture';
import { PerspectiveSelector } from '../components/chat/PerspectiveSelector';
import { Modal } from '../components/common/Modal';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import type { Session, SessionEvent, SessionSummary, Perspective, Character, CreateSessionEventRequest } from '../api/types';
import { deriveSessionStatus } from '../api/types';

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

  const [tab, setTab] = useState<'events' | 'summary' | 'debrief' | 'loot' | 'map' | 'voice'>('events');
  const [voiceTranscript, setVoiceTranscript] = useState('');
  const [showAddEvent, setShowAddEvent] = useState(false);
  const [perspective, setPerspective] = useState<Perspective>('dm');
  const [summary, setSummary] = useState<SessionSummary | null>(null);
  const [summaryLoading, setSummaryLoading] = useState(false);
  const [summaryError, setSummaryError] = useState('');
  const [actionError, setActionError] = useState('');
  const [eventError, setEventError] = useState('');

  // Map upload state
  const mapFileInputRef = useRef<HTMLInputElement>(null);
  const [mapUploading, setMapUploading] = useState(false);
  const [mapError, setMapError] = useState('');

  // Debrief state
  const [debriefText, setDebriefText] = useState<string | null>(null);
  const [debriefLoading, setDebriefLoading] = useState(false);
  const [debriefError, setDebriefError] = useState('');
  const [showDebriefPanel, setShowDebriefPanel] = useState(false);

  useEffect(() => { setSummary(null); }, [perspective]);

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
    setEventError('');
    try {
      await createEvent(campaignId!, sessionId!, data);
      setShowAddEvent(false);
      refetchEvents();
    } catch (e: unknown) {
      setEventError(e instanceof Error ? e.message : String(e));
    }
  };

  const handleDeleteEvent = async (eventId: string) => {
    try {
      await deleteEvent(campaignId!, sessionId!, eventId);
      refetchEvents();
    } catch (e: unknown) {
      setActionError(e instanceof Error ? e.message : String(e));
    }
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

  const handleGenerateDebrief = async () => {
    setDebriefLoading(true);
    setDebriefError('');
    setDebriefText(null);
    try {
      const result = await generateDebrief(campaignId!, sessionId!);
      setDebriefText(result.debrief);
      setShowDebriefPanel(true);
    } catch (e: unknown) {
      setDebriefError(e instanceof Error ? e.message : String(e));
      setShowDebriefPanel(true);
    } finally {
      setDebriefLoading(false);
    }
  };

  const handleMapUpload = async (file: File) => {
    setMapUploading(true);
    setMapError('');
    try {
      await uploadSessionMap(campaignId!, sessionId!, file);
      refetchSession();
    } catch (e: unknown) {
      setMapError(e instanceof Error ? e.message : String(e));
    } finally {
      setMapUploading(false);
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
          {deriveSessionStatus(session) === 'pending' && (
            <button className="btn btn-primary" onClick={handleStart}>Start Session</button>
          )}
          {deriveSessionStatus(session) === 'started' && (
            <button className="btn btn-danger" onClick={handleEnd}>End Session</button>
          )}
          {deriveSessionStatus(session) === 'ended' && <span className="badge badge-default">Ended</span>}
        </div>
      </div>

      {actionError && <ErrorBanner message={actionError} />}

      <div className="tab-nav">
        <button className={`tab-link ${tab === 'events' ? 'active' : ''}`} onClick={() => setTab('events')}>Events</button>
        <button className={`tab-link ${tab === 'summary' ? 'active' : ''}`} onClick={() => setTab('summary')}>Summary</button>
        <button className={`tab-link ${tab === 'debrief' ? 'active' : ''}`} onClick={() => setTab('debrief')}>Debrief</button>
        <button className={`tab-link ${tab === 'loot' ? 'active' : ''}`} onClick={() => setTab('loot')}>Loot</button>
        <button className={`tab-link ${tab === 'map' ? 'active' : ''}`} onClick={() => setTab('map')}>Map</button>
        <button className={`tab-link ${tab === 'voice' ? 'active' : ''}`} onClick={() => setTab('voice')}>Voice</button>
      </div>

      {tab === 'events' && (
        <div>
          <div className="section-header">
            <h3>Events</h3>
            <button className="btn btn-sm btn-primary" onClick={() => setShowAddEvent(true)}>+ Add Event</button>
          </div>
          {events && <SessionEventList events={events} onDelete={handleDeleteEvent} />}

          {showAddEvent && (
            <Modal title="Add Event" onClose={() => { setShowAddEvent(false); setVoiceTranscript(''); }}>
              {eventError && <ErrorBanner message={eventError} />}
              {voiceTranscript && (
                <div style={{ marginBottom: 8, padding: 8, background: 'var(--color-surface-2, #2a2a3e)', borderRadius: 4, fontSize: 13, color: '#ccc' }}>
                  <strong>Voice transcript:</strong> {voiceTranscript}
                </div>
              )}
              <SessionEventForm
                characters={characters ?? []}
                onSubmit={handleAddEvent}
                onCancel={() => { setShowAddEvent(false); setVoiceTranscript(''); }}
              />
            </Modal>
          )}
        </div>
      )}

      {tab === 'summary' && (
        <div className="summary-tab">
          <div className="summary-controls">
            <PerspectiveSelector value={perspective} onChange={setPerspective} disabled={summaryLoading} />
            <button
              className="btn btn-primary"
              onClick={handleGenerateSummary}
              disabled={summaryLoading || !events || events.length === 0}
              title={(!events || events.length === 0) ? 'Add events before generating a summary' : undefined}
            >
              {summaryLoading ? <><LoadingSpinner size={14} /> Generating...</> : 'Generate Summary'}
            </button>
          </div>
          {summaryError && <ErrorBanner message={summaryError} />}
          {summary && <SummaryView summary={summary} />}
          {!summary && !summaryLoading && !summaryError && (
            <p className="empty-state">Click "Generate Summary" to create an AI-powered session recap.</p>
          )}
        </div>
      )}

      {tab === 'debrief' && (
        <div className="debrief-tab">
          <div className="section-header">
            <div>
              <h3>AI Session Debrief</h3>
              <p className="section-description" style={{ margin: '0.25rem 0 0', color: 'var(--color-text-muted, #666)', fontSize: '0.875rem' }}>
                Get a structured post-session analysis covering what went well, what was confusing, open plot threads, and next session suggestions.
              </p>
            </div>
            <button
              className="btn btn-primary"
              onClick={handleGenerateDebrief}
              disabled={debriefLoading}
            >
              {debriefLoading ? <><LoadingSpinner size={14} /> Generating...</> : 'Generate Debrief'}
            </button>
          </div>

          {showDebriefPanel && (
            <div className="debrief-panel" style={{ marginTop: '1rem' }}>
              {debriefError && <ErrorBanner message={debriefError} />}
              {debriefText && (
                <div className="debrief-content card" style={{ padding: '1.5rem', background: 'var(--color-surface, #1e1e2e)', borderRadius: '0.5rem', whiteSpace: 'pre-wrap', lineHeight: '1.6' }}>
                  {debriefText}
                </div>
              )}
            </div>
          )}

          {!showDebriefPanel && !debriefLoading && (
            <p className="empty-state">Click "Generate Debrief" to receive an AI-powered post-session coaching report.</p>
          )}
        </div>
      )}

      {tab === 'loot' && (
        <LootLog campaignId={campaignId!} sessionId={sessionId!} />
      )}

      {tab === 'map' && (
        <div className="map-tab">
          <div className="section-header">
            <h3>Session Map</h3>
            <div>
              <input
                ref={mapFileInputRef}
                type="file"
                accept="image/*"
                style={{ display: 'none' }}
                onChange={(e) => {
                  const file = e.target.files?.[0];
                  if (file) handleMapUpload(file);
                }}
              />
              <button
                className="btn btn-primary"
                onClick={() => mapFileInputRef.current?.click()}
                disabled={mapUploading}
              >
                {mapUploading ? <><LoadingSpinner size={14} /> Uploading...</> : 'Upload Map'}
              </button>
            </div>
          </div>
          {mapError && <ErrorBanner message={mapError} />}
          {session.map_url ? (
            <div className="map-container" style={{ marginTop: '1rem' }}>
              <img
                src={`http://localhost:8000${session.map_url}`}
                alt="Session map"
                style={{ maxWidth: '100%', borderRadius: '0.5rem', border: '1px solid var(--color-border, #333)' }}
              />
            </div>
          ) : (
            <p className="empty-state">No map uploaded for this session yet.</p>
          )}
        </div>
      )}

      {tab === 'voice' && (
        <div className="voice-tab">
          <div className="section-header">
            <h3>Voice Notes</h3>
          </div>
          <p style={{ color: '#aaa', fontSize: 13, marginBottom: 12 }}>
            Speak session notes. Save as an event to add them to the session log.
          </p>
          <VoiceNoteCapture
            campaignId={campaignId!}
            sessionId={sessionId!}
            onTranscript={(text) => {
              setVoiceTranscript(text);
              setShowAddEvent(true);
            }}
          />
        </div>
      )}
    </div>
  );
}
