│ Plan: Review & Complete Known Gaps / Polish Items │
│ │
│ Context │
│ │
│ TODO.md tracks implementation status. Several "Known Gaps / Polish Items" are already done but marked incomplete. The remaining │
│ gaps are: character delete UI, document ingest completion callback, and a stale dev-dependency. There's also an untracked migration │
│ file and backend clippy check pending. │
│ │
│ --- │
│ Phase 1: Mark Completed Items in TODO.md │
│ │
│ Update TODO.md to check off these items (already verified done): │
│ - CampaignDetailPage nested routing — NavLink tabs + Outlet + correct App.tsx routes │
│ - deleteSession — exported from sessions.ts, imported and used in SessionsPage.tsx │
│ - Encounter encId param — useParams and route definition both use :encId │
│ - useCampaign error path — rejects with error when campaignId is undefined │
│ │
│ File: TODO.md (lines 232–237, Known Gaps section) │
│ │
│ --- │
│ Phase 2: Fix @types/react-router-dom Version Mismatch │
│ │
│ Remove the stale @types/react-router-dom@5.3.3 package — react-router-dom v7 ships its own types. │
│ │
│ Command: cd guide-frontend && bun remove @types/react-router-dom │
│ │
│ Files affected: guide-frontend/package.json, guide-frontend/bun.lock │
│ │
│ --- │
│ Phase 3: Fix Document Ingest Polling Refetch │
│ │
│ Add an onComplete prop to IngestButton that fires when polling resolves to completed. │
│ │
│ 3a. Update IngestButton.tsx │
│ │
│ File: guide-frontend/src/components/documents/IngestButton.tsx │
│ │
│ Add optional onComplete?: () => void to the props interface. Call it inside the interval callback when status becomes completed: │
│ if (doc.ingestion_status === 'completed') { │
│ clearPoller(); │
│ onComplete?.(); │
│ } else if (doc.ingestion_status === 'failed') { │
│ clearPoller(); │
│ } │
│ │
│ 3b. Wire onComplete in DocumentsPage.tsx │
│ │
│ File: guide-frontend/src/pages/DocumentsPage.tsx │
│ │
│ Pass onComplete={refetch} to the <IngestButton> render in the renderActions callback. This triggers a list refetch when ingest │
│ completes, updating all document statuses in the table. │
│ │
│ --- │
│ Phase 4: Add Character Delete Button │
│ │
│ 4a. CharacterDetailPage.tsx │
│ │
│ File: guide-frontend/src/pages/CharacterDetailPage.tsx │
│ │
│ - Import deleteCharacter from ../api/characters and ConfirmButton from ../components/common/ConfirmButton │
│ - Add a delete handler that calls deleteCharacter(campaignId!, charId!) then navigates back to /campaigns/${campaignId}/characters │
│ - Add a <ConfirmButton> at the bottom of the page (or in the page header) with onConfirm={handleDelete} and label "Delete │
│ Character" │
│ │
│ 4b. CharacterCard.tsx (optional but recommended) │
│ │
│ File: guide-frontend/src/components/characters/CharacterCard.tsx │
│ │
│ - Add onDelete?: (id: string) => void prop │
│ - Add a <ConfirmButton> in the card footer with onConfirm={() => onDelete?.(character.id)} │
│ - Wire onDelete={handleDelete} in CharacterList → CharactersPage │
│ │
│ --- │
│ Phase 5: Track Migration 007 │
│ │
│ Migration crates/guide-db/migrations/007_encounter_optional_session.sql is untracked. It should simply be staged — no code changes │
│ needed. │
│ │
│ --- │
│ Phase 6: Run cargo clippy │
│ │
│ Command: cargo clippy --workspace -- -D warnings │
│ │
│ Fix any warnings that arise. Mark the clippy item in TODO.md Step 12 as complete once clean. │
│ │
│ --- │
│ Verification │
│ │
│ 1. cd guide-frontend && bun run build — must compile clean (no TypeScript errors after removing stale types) │
│ 2. cargo clippy --workspace -- -D warnings — zero errors │
│ 3. cargo test --workspace — 55/55 tests still pass │
│ 4. Manual smoke test: upload a document, trigger ingest, verify document list refreshes automatically when ingest completes │
│ 5. Manual smoke test: navigate to a character detail page, confirm delete button present and works (redirects back to list) │
│ │
│ --- │
│ Files to Modify │
│ │
│ ┌───────────────────────────────────────────────────────────────┬────────────────────────────────────┐ │
│ │ File │ Change │ │
│ ├───────────────────────────────────────────────────────────────┼────────────────────────────────────┤ │
│ │ TODO.md │ Mark 4 items as done in Known Gaps │ │
│ ├───────────────────────────────────────────────────────────────┼────────────────────────────────────┤ │
│ │ guide-frontend/package.json + bun.lock │ Remove @types/react-router-dom │ │
│ ├───────────────────────────────────────────────────────────────┼────────────────────────────────────┤ │
│ │ guide-frontend/src/components/documents/IngestButton.tsx │ Add onComplete prop │ │
│ ├───────────────────────────────────────────────────────────────┼────────────────────────────────────┤ │
│ │ guide-frontend/src/pages/DocumentsPage.tsx │ Pass onComplete={refetch} │ │
│ ├───────────────────────────────────────────────────────────────┼────────────────────────────────────┤ │
│ │ guide-frontend/src/pages/CharacterDetailPage.tsx │ Add delete handler + ConfirmButton │ │
│ ├───────────────────────────────────────────────────────────────┼────────────────────────────────────┤ │
│ │ guide-frontend/src/components/characters/CharacterCard.tsx │ Add optional onDelete prop │ │
│ ├───────────────────────────────────────────────────────────────┼────────────────────────────────────┤ │
│ │ guide-frontend/src/components/characters/CharacterList.tsx │ Wire onDelete through │ │
│ ├───────────────────────────────────────────────────────────────┼────────────────────────────────────┤ │
│ │ guide-frontend/src/pages/CharactersPage.tsx │ Pass handleDelete │ │
│ ├───────────────────────────────────────────────────────────────┼────────────────────────────────────┤ │
│ │ crates/guide-db/migrations/007_encounter_optional_session.sql │ Stage/commit (no edits) │ │
│ └───────────────────────────────────────────────────────────────┴────────────────────────────────────┘ │
