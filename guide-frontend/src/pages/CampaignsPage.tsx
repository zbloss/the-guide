import { useState } from 'react';
import { useApi } from '../hooks/useApi';
import { listCampaigns, createCampaign, deleteCampaign } from '../api/campaigns';
import { CampaignCard } from '../components/campaigns/CampaignCard';
import { CampaignForm } from '../components/campaigns/CampaignForm';
import { Modal } from '../components/common/Modal';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import type { Campaign, CreateCampaignRequest } from '../api/types';

export function CampaignsPage() {
  const { data: campaigns, loading, error, refetch } = useApi<Campaign[]>(listCampaigns, []);
  const [showCreate, setShowCreate] = useState(false);

  const handleCreate = async (data: CreateCampaignRequest) => {
    await createCampaign(data);
    setShowCreate(false);
    refetch();
  };

  const handleDelete = async (id: string) => {
    await deleteCampaign(id);
    refetch();
  };

  return (
    <div className="page">
      <div className="page-header">
        <h1>Campaigns</h1>
        <button className="btn btn-primary" onClick={() => setShowCreate(true)}>
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

      {showCreate && (
        <Modal title="New Campaign" onClose={() => setShowCreate(false)}>
          <CampaignForm onSubmit={handleCreate} onCancel={() => setShowCreate(false)} />
        </Modal>
      )}
    </div>
  );
}
