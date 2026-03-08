import { CharacterCard } from './CharacterCard';
import type { Character } from '../../api/types';

interface CharacterListProps {
  characters: Character[];
  campaignId: string;
}

export function CharacterList({ characters, campaignId }: CharacterListProps) {
  if (characters.length === 0) {
    return <p className="empty-state">No characters yet. Add one to get started.</p>;
  }
  return (
    <div className="character-grid">
      {characters.map((c) => (
        <CharacterCard key={c.id} character={c} campaignId={campaignId} />
      ))}
    </div>
  );
}
