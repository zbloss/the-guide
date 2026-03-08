import { useParams, NavLink, Outlet } from 'react-router-dom';
import { useCampaign } from '../hooks/useCampaign';
import { WorldStateEditor } from '../components/campaigns/WorldStateEditor';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import { Badge } from '../components/common/Badge';
import type { Campaign } from '../api/types';

export function CampaignDetailPage() {
  const { campaignId } = useParams<{ campaignId: string }>();
  const { campaign, loading, error, refetch } = useCampaign(campaignId);

  if (loading) return <div className="page"><LoadingSpinner /></div>;
  if (error) return <div className="page"><ErrorBanner message={error} /></div>;
  if (!campaign) return null;

  const tabs = [
    { label: 'Characters', to: 'characters' },
    { label: 'Sessions', to: 'sessions' },
    { label: 'Encounters', to: 'encounters' },
    { label: 'Documents', to: 'documents' },
    { label: 'Chat', to: 'chat' },
  ];

  return (
    <div className="page">
      <div className="page-header">
        <h1>{campaign.name}</h1>
        <Badge label={campaign.game_system} variant="info" />
      </div>

      {campaign.description && <p className="campaign-description">{campaign.description}</p>}

      <WorldStateEditor campaign={campaign} onSaved={(updated: Campaign) => { void updated; refetch(); }} />

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
