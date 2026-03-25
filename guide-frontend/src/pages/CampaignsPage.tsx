import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { listCampaigns, deleteCampaign } from '../api/campaigns';
import { CampaignCard } from '../components/campaigns/CampaignCard';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import { cacheItems, getCachedItems } from '../lib/offlineDb';
import { useNetworkStatus } from '../hooks/useNetworkStatus';
import type { Campaign } from '../api/types';

export function CampaignsPage() {
  const navigate = useNavigate();
  const { isOnline } = useNetworkStatus();
  const [campaigns, setCampaigns] = useState<Campaign[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [tick, setTick] = useState(0);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    listCampaigns()
      .then((result) => {
        if (!cancelled) {
          setCampaigns(result);
          setLoading(false);
          cacheItems('campaigns', result).catch(console.error);
        }
      })
      .catch(async (err: unknown) => {
        if (!cancelled) {
          const cached = await getCachedItems<Campaign>('campaigns');
          if (cached.length > 0) {
            setCampaigns(cached);
          } else {
            setError(err instanceof Error ? err.message : String(err));
          }
          setLoading(false);
        }
      });
    return () => { cancelled = true; };
  }, [tick]);

  const handleDelete = async (id: string) => {
    try {
      await deleteCampaign(id);
      setTick((t) => t + 1);
      window.dispatchEvent(new CustomEvent('campaigns-changed'));
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div className="page">
      <div className="page-header">
        <h1>Campaigns</h1>
        <button
          className="btn btn-primary"
          onClick={() => navigate('/campaigns/new')}
          disabled={!isOnline}
          title={!isOnline ? 'Unavailable offline' : undefined}
        >
          + New Campaign
        </button>
      </div>

      {loading && <LoadingSpinner />}
      {error && <ErrorBanner message={error} />}

      {campaigns && (
        <div className="campaign-grid">
          {campaigns.length === 0 && (
            <p className="empty-state">No campaigns yet. Create one to get started!</p>
          )}
          {campaigns.map((c) => (
            <CampaignCard key={c.id} campaign={c} onDelete={handleDelete} />
          ))}
        </div>
      )}
    </div>
  );
}
