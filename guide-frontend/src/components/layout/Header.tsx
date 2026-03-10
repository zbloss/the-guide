import { useEffect, useState } from 'react';
import { getHealth } from '../../api/health';

interface HeaderProps {
  title?: string;
}

function useTheme() {
  const [theme, setTheme] = useState<'dark' | 'light'>(() => {
    return (localStorage.getItem('theme') as 'dark' | 'light') ?? 'dark';
  });

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('theme', theme);
  }, [theme]);

  const toggle = () => setTheme((t) => (t === 'dark' ? 'light' : 'dark'));
  return { theme, toggle };
}

export function Header({ title }: HeaderProps) {
  const [backendOk, setBackendOk] = useState<boolean | null>(null);
  const { theme, toggle } = useTheme();

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
        <button
          className="btn btn-sm theme-toggle"
          onClick={toggle}
          title={`Switch to ${theme === 'dark' ? 'light' : 'dark'} mode`}
          aria-label="Toggle theme"
        >
          {theme === 'dark' ? '☀️' : '🌙'}
        </button>
        <span className={`status-pill ${backendOk === true ? 'status-ok' : backendOk === false ? 'status-err' : 'status-unknown'}`}>
          {backendOk === true ? 'Backend ✓' : backendOk === false ? 'Backend ✗' : 'Connecting…'}
        </span>
      </div>
    </header>
  );
}
