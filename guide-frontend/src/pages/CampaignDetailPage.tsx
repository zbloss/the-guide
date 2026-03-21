import { useState, useEffect } from 'react';
import { useParams, NavLink, Outlet } from 'react-router-dom';
import { useCampaign } from '../hooks/useCampaign';
import { updateCampaign } from '../api/campaigns';
import { WorldStateEditor } from '../components/campaigns/WorldStateEditor';
import { ConsistencyReportPanel } from '../components/campaigns/ConsistencyReportPanel';
import { CampaignSearch } from '../components/campaigns/CampaignSearch';
import { HomebrewRuleList } from '../components/campaigns/HomebrewRuleList';
import { FactionTracker } from '../components/campaigns/FactionTracker';
import { PlotTwistModal } from '../components/campaigns/PlotTwistModal';
import { WebhookManager } from '../components/campaigns/WebhookManager';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import { Badge } from '../components/common/Badge';
import type { Campaign, GameSystem } from '../api/types';
import { cacheItems } from '../lib/offlineDb';

export function CampaignDetailPage() {
  const { campaignId } = useParams<{ campaignId: string }>();
  const { campaign, loading, error, refetch } = useCampaign(campaignId);

  useEffect(() => {
    if (campaign) {
      cacheItems('campaigns', [campaign]).catch(console.error);
    }
  }, [campaign]);

  const [editingMeta, setEditingMeta] = useState(false);
  const [editName, setEditName] = useState('');
  const [editDescription, setEditDescription] = useState('');
  const [editGameSystem, setEditGameSystem] = useState<GameSystem>('dnd5e');
  const [metaSaving, setMetaSaving] = useState(false);
  const [metaError, setMetaError] = useState('');
  if (loading) return <div className="page"><LoadingSpinner /></div>;
  if (error) return <div className="page"><ErrorBanner message={error} /></div>;
  if (!campaign) return null;

  const handleEditOpen = () => {
    setEditName(campaign.name);
    setEditDescription(campaign.description ?? '');
    setEditGameSystem(campaign.game_system);
    setMetaError('');
    setEditingMeta(true);
  };

  const handleMetaSave = async () => {
    setMetaSaving(true);
    setMetaError('');
    try {
      await updateCampaign(campaignId!, {
        name: editName,
        description: editDescription || undefined,
        game_system: editGameSystem,
      });
      setEditingMeta(false);
      refetch();
    } catch (e: unknown) {
      setMetaError(e instanceof Error ? e.message : String(e));
    } finally {
      setMetaSaving(false);
    }
  };

  const tabs = [
    { label: 'Story', to: 'story' },
    { label: 'Characters', to: 'characters' },
    { label: 'Sessions', to: 'sessions' },
    { label: 'Encounters', to: 'encounters' },
    { label: 'Documents', to: 'documents' },
    { label: 'Chat', to: 'chat' },
    { label: 'Analytics', to: 'analytics' },
    { label: 'Prep', to: 'prep' },
    { label: 'Relationships', to: 'relationships' },
  ];

  return (
    <div className="page">
      <div className="page-header">
        {editingMeta ? (
          <div className="meta-edit-form">
            {metaError && <ErrorBanner message={metaError} />}
            <div className="form-row">
              <div className="form-field" style={{ flex: 2 }}>
                <label className="form-label">Name</label>
                <input className="form-input" value={editName} onChange={(e) => setEditName(e.target.value)} />
              </div>
              <div className="form-field">
                <label className="form-label">System</label>
                <select className="form-input" value={editGameSystem} onChange={(e) => setEditGameSystem(e.target.value as GameSystem)}>
                  <option value="dnd5e">D&D 5e</option>
                  <option value="pathfinder2e">Pathfinder 2e</option>
                  <option value="custom">Custom</option>
                </select>
              </div>
            </div>
            <div className="form-field">
              <label className="form-label">Description</label>
              <textarea className="form-input" value={editDescription} onChange={(e) => setEditDescription(e.target.value)} rows={2} />
            </div>
            <div className="form-actions">
              <button className="btn btn-primary btn-sm" onClick={handleMetaSave} disabled={metaSaving}>{metaSaving ? 'Saving…' : 'Save'}</button>
              <button className="btn btn-sm" onClick={() => setEditingMeta(false)} disabled={metaSaving}>Cancel</button>
            </div>
          </div>
        ) : (
          <>
            <h1>{campaign.name}</h1>
            <Badge label={campaign.game_system} variant="info" />
            <button className="btn btn-sm" onClick={handleEditOpen}>Edit</button>
            <PlotTwistModal campaignId={campaignId!} />
          </>
        )}
      </div>

      {!editingMeta && campaign.description && <p className="campaign-description">{campaign.description}</p>}

      <WorldStateEditor campaign={campaign} onSaved={(updated: Campaign) => { void updated; refetch(); }} />

      <CampaignSearch campaignId={campaignId!} />
      <ConsistencyReportPanel campaignId={campaignId!} />
      <FactionTracker campaignId={campaignId!} />
      <HomebrewRuleList campaignId={campaignId!} />
      <WebhookManager campaignId={campaignId!} />

      <nav className="tab-nav">
        {tabs.map((tab) => (
          <NavLink
            key={tab.to}
            to={tab.to}
            className={({ isActive }) => `tab-link ${isActive ? 'active' : ''}`}
          >
            {tab.label}
          </NavLink>
        ))}
      </nav>

      <Outlet />
    </div>
  );
}
