import { useState } from 'react';
import { useParams } from 'react-router-dom';
import { useApi } from '../hooks/useApi';
import { listCharacters, createCharacter } from '../api/characters';
import { CharacterList } from '../components/characters/CharacterList';
import { CharacterForm } from '../components/characters/CharacterForm';
import { Modal } from '../components/common/Modal';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import type { Character, CreateCharacterRequest } from '../api/types';

export function CharactersPage() {
  const { campaignId } = useParams<{ campaignId: string }>();
  const { data: characters, loading, error, refetch } = useApi<Character[]>(
    () => listCharacters(campaignId!),
    [campaignId],
  );
  const [showCreate, setShowCreate] = useState(false);

  const handleCreate = async (data: CreateCharacterRequest) => {
    await createCharacter(campaignId!, data);
    setShowCreate(false);
    refetch();
  };

  return (
    <div className="page-section">
      <div className="section-header">
        <h2>Characters</h2>
        <button className="btn btn-primary" onClick={() => setShowCreate(true)}>+ Add Character</button>
      </div>

      {loading && <LoadingSpinner />}
      {error && <ErrorBanner message={error} />}
      {characters && <CharacterList characters={characters} campaignId={campaignId!} />}

      {showCreate && (
        <Modal title="New Character" onClose={() => setShowCreate(false)}>
          <CharacterForm onSubmit={handleCreate} onCancel={() => setShowCreate(false)} />
        </Modal>
      )}
    </div>
  );
}
