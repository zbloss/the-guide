import { useParams } from 'react-router-dom';
import { useApi } from '../hooks/useApi';
import { getSharedView } from '../api/campaigns';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';

interface PartyMember { name: string; class: string | null; race: string | null; level: number | null; portrait_url: string | null; }
interface SharedView {
  campaign: { id: string; name: string; description: string | null; game_system: string; world_state: { current_location?: string; active_quests?: string[] } | null };
  party: PartyMember[];
  last_session: { session_number: number; title: string | null; created_at: string } | null;
  session_count: number;
}

export function SharedViewPage() {
  const { token } = useParams<{ token: string }>();
  const { data, loading, error } = useApi<SharedView>(
    () => getSharedView(token!) as Promise<SharedView>,
    [token],
  );

  if (loading) return <div style={{ padding: 40 }}><LoadingSpinner /></div>;
  if (error) return <div style={{ padding: 40 }}><ErrorBanner message={error} /></div>;
  if (!data) return null;

  const { campaign, party, last_session, session_count } = data;

  return (
    <div className="shared-view">
      <div className="shared-view-header">
        <h1 className="shared-view-title">{campaign.name}</h1>
        <span className="badge badge-info">{campaign.game_system}</span>
        {campaign.description && <p className="shared-view-desc">{campaign.description}</p>}
      </div>

      {campaign.world_state?.current_location && (
        <div className="shared-view-section">
          <h2>Current Location</h2>
          <p>{campaign.world_state.current_location}</p>
        </div>
      )}

      {campaign.world_state?.active_quests && campaign.world_state.active_quests.length > 0 && (
        <div className="shared-view-section">
          <h2>Active Quests</h2>
          <ul>{campaign.world_state.active_quests.map((q, i) => <li key={i}>{q}</li>)}</ul>
        </div>
      )}

      <div className="shared-view-section">
        <h2>Party ({party.length} members)</h2>
        <div className="shared-party-grid">
          {party.map((m, i) => (
            <div key={i} className="shared-party-card">
              {m.portrait_url ? (
                <img src={m.portrait_url} alt={m.name} className="shared-portrait" />
              ) : (
                <div className="shared-portrait-placeholder">{m.name[0]}</div>
              )}
              <div className="shared-party-name">{m.name}</div>
              <div className="shared-party-meta">
                {[m.race, m.class, m.level ? `Lvl ${m.level}` : null].filter(Boolean).join(' · ')}
              </div>
            </div>
          ))}
        </div>
      </div>

      <div className="shared-view-section">
        <h2>Campaign Progress</h2>
        <p>{session_count} session{session_count !== 1 ? 's' : ''} played</p>
        {last_session && (
          <p className="text-muted" style={{ marginTop: 4 }}>
            Last: Session {last_session.session_number}{last_session.title ? ` — ${last_session.title}` : ''}
          </p>
        )}
      </div>

      <footer className="shared-view-footer">
        Powered by <strong>The Guide</strong>
      </footer>
    </div>
  );
}
