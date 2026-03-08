import { Outlet, useParams } from 'react-router-dom';
import { Sidebar } from './Sidebar';
import { Header } from './Header';
import { useCampaign } from '../../hooks/useCampaign';

function CampaignHeader() {
  const { campaignId } = useParams<{ campaignId: string }>();
  const { campaign } = useCampaign(campaignId);
  return <Header title={campaign?.name} />;
}

export function Layout() {
  return (
    <div className="app-layout">
      <Sidebar />
      <div className="app-main">
        <CampaignHeader />
        <main className="app-content">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
