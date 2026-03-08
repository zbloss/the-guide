import { useEffect, useState } from 'react';
import { getHealth } from '../../api/health';

interface HeaderProps {
  title?: string;
}

export function Header({ title }: HeaderProps) {
  const [backendOk, setBackendOk] = useState<boolean | null>(null);

  useEffect(() => {
    const check = () => {
      getHealth()
        .then(() => setBackendOk(true))
        .catch(() => setBackendOk(false));
    };
    check();
    const id = setInterval(check, 30_000);
    return () => clearInterval(id);
  }, []);

  return (
    <header className="header">
      <div className="header-title">{title ?? 'The Guide'}</div>
      <div className="header-status">
        <span className={`status-pill ${backendOk === true ? 'status-ok' : backendOk === false ? 'status-err' : 'status-unknown'}`}>
          {backendOk === true ? 'Backend ✓' : backendOk === false ? 'Backend ✗' : 'Connecting…'}
        </span>
      </div>
    </header>
  );
}
