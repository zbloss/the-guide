import { Routes, Route } from "react-router-dom";
import { Layout } from "./components/layout/Layout";
import { CampaignsPage } from "./pages/CampaignsPage";
import { CampaignDetailPage } from "./pages/CampaignDetailPage";
import { CharactersPage } from "./pages/CharactersPage";
import { CharacterDetailPage } from "./pages/CharacterDetailPage";
import { SessionsPage } from "./pages/SessionsPage";
import { SessionDetailPage } from "./pages/SessionDetailPage";
import { EncountersPage } from "./pages/EncountersPage";
import { EncounterDetailPage } from "./pages/EncounterDetailPage";
import { DocumentsPage } from "./pages/DocumentsPage";
import { GlobalDocumentsPage } from "./pages/GlobalDocumentsPage";
import { ChatPage } from "./pages/ChatPage";
import { AnalyticsPage } from "./pages/AnalyticsPage";
import { HealthPage } from "./pages/HealthPage";
import { NotFoundPage } from "./pages/NotFoundPage";
import { PlaystylePage } from "./pages/PlaystylePage";
import { PrepPage } from "./pages/PrepPage";
import { RelationshipMapPage } from "./pages/RelationshipMapPage";
import { StoryPage } from "./pages/StoryPage";
import { CampaignWizardPage } from "./pages/CampaignWizardPage";
import { AdminPage } from "./pages/AdminPage";

function App() {
  return (
    <Routes>
      <Route element={<Layout />}>
        <Route index element={<CampaignsPage />} />

        <Route path="campaigns/new" element={<CampaignWizardPage />} />
        <Route path="campaigns/:campaignId" element={<CampaignDetailPage />}>
          <Route path="story" element={<StoryPage />} />
          <Route path="characters" element={<CharactersPage />} />
          <Route path="characters/:charId" element={<CharacterDetailPage />} />
          <Route path="sessions" element={<SessionsPage />} />
          <Route path="sessions/:sessionId" element={<SessionDetailPage />} />
          <Route path="encounters" element={<EncountersPage />} />
          <Route path="encounters/:encId" element={<EncounterDetailPage />} />
          <Route path="documents" element={<DocumentsPage />} />
          <Route path="chat" element={<ChatPage />} />
          <Route path="analytics" element={<AnalyticsPage />} />
          <Route path="prep" element={<PrepPage />} />
          <Route path="relationships" element={<RelationshipMapPage />} />
        </Route>

        <Route path="documents" element={<GlobalDocumentsPage />} />
        <Route path="playstyle" element={<PlaystylePage />} />
        <Route path="health" element={<HealthPage />} />
        <Route path="admin" element={<AdminPage />} />
        <Route path="*" element={<NotFoundPage />} />
      </Route>
    </Routes>
  );
}

export default App;
