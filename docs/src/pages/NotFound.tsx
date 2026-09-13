// In-app 404, mirroring the prerendered 404.html.
import { Link } from "react-router-dom";

export default function NotFound() {
  return (
    <div className="dot-backdrop">
      <div className="center-wrap">
        <p className="eyebrow">Error 404 / Route unknown</p>
        <h1 className="grad-text">Page not found</h1>
        <p className="center-lede">
          This OrbyNode documentation page could not be found. It may have
          moved, or the link that brought you here is out of date.
        </p>
        <div className="center-cta">
          <Link to="/docs" className="btn btn-primary">
            Browse documentation
          </Link>
          <Link to="/" className="btn btn-ghost">
            Back home
          </Link>
        </div>
      </div>
    </div>
  );
}
