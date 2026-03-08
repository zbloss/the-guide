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
import { HealthPage } from "./pages/HealthPage";
import { NotFoundPage } from "./pages/NotFoundPage";

function App() {
  return (
    <Routes>
      <Route element={<Layout />}>
        <Route index element={<CampaignsPage />} />

        <Route path="campaigns/:campaignId" element={<CampaignDetailPage />}>
          <Route path="characters" element={<CharactersPage />} />
          <Route path="characters/:charId" element={<CharacterDetailPage />} />
          <Route path="sessions" element={<SessionsPage />} />
          <Route path="sessions/:sessionId" element={<SessionDetailPage />} />
          <Route path="encounters" element={<EncountersPage />} />
          <Route path="encounters/:encId" element={<EncounterDetailPage />} />
          <Route path="documents" element={<DocumentsPage />} />
          <Route path="chat" element={<ChatPage />} />
        </Route>

        <Route path="documents" element={<GlobalDocumentsPage />} />
        <Route path="health" element={<HealthPage />} />
        <Route path="*" element={<NotFoundPage />} />
      </Route>
    </Routes>
  );
}

export default App;
