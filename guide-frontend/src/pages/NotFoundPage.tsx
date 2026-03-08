import { Link } from 'react-router-dom';

export function NotFoundPage() {
  return (
    <div className="page not-found-page">
      <h1>404 — Page Not Found</h1>
      <p>The page you're looking for doesn't exist.</p>
      <Link to="/" className="btn btn-primary">Go to Campaigns</Link>
    </div>
  );
}
