import { useEffect } from 'react';
import { Outlet, useParams } from 'react-router-dom';
import { Sidebar } from './Sidebar';
import { Header } from './Header';
import { RuleReferenceSidebar } from '../common/RuleReferenceSidebar';
import { OfflineIndicator } from '../common/OfflineIndicator';
import { useCampaign } from '../../hooks/useCampaign';
import { useNetworkStatus } from '../../hooks/useNetworkStatus';
import { syncPendingWrites } from '../../lib/syncManager';

function CampaignHeader() {
  const { campaignId } = useParams<{ campaignId: string }>();
  const { campaign } = useCampaign(campaignId);
  return <Header title={campaignId ? campaign?.name : undefined} />;
}

export function Layout() {
  const { isOnline } = useNetworkStatus();

  useEffect(() => {
    if (isOnline) {
      syncPendingWrites().catch(console.error);
    }
  }, [isOnline]);

  return (
    <div className="app-layout">
      <OfflineIndicator />
      <Sidebar />
      <div className="app-main">
        <CampaignHeader />
        <main className="app-content">
          <Outlet />
        </main>
      </div>
      <RuleReferenceSidebar />
    </div>
  );
}
