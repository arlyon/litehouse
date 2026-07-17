import { Moon, Sun } from "lucide-react";
import { useState } from "react";
import { CapabilityTable } from "./CapabilityTable";
import { capabilityFixture, recordFixtureIntent } from "./fixtures";
import type { CapabilityFacet, CapabilityIntent } from "./model";

export function CapabilityReference() {
  const [facets, setFacets] = useState<readonly CapabilityFacet[]>(
    capabilityFixture,
  );
  const [dark, setDark] = useState(false);
  const [lastIntent, setLastIntent] = useState<CapabilityIntent["type"] | null>(
    null,
  );

  function dispatch(intent: CapabilityIntent) {
    setFacets((current) => recordFixtureIntent(current, intent));
    setLastIntent(intent.type);
  }

  function toggleTheme() {
    setDark((current) => {
      const next = !current;
      const root = document.documentElement;
      root.classList.toggle("dark", next);
      root.dataset.theme = next ? "dark" : "light";
      root.style.colorScheme = next ? "dark" : "light";
      return next;
    });
  }

  return (
    <main id="main-content" className="app-shell" tabIndex={-1}>
      <header className="app-header">
        <span className="wordmark">
          Litehouse <span>System</span>
        </span>
        <button
          className="theme-toggle"
          type="button"
          aria-pressed={dark}
          onClick={toggleTheme}
        >
          {dark ? <Sun aria-hidden="true" /> : <Moon aria-hidden="true" />}
          {dark ? "Use light mode" : "Use dark mode"}
        </button>
      </header>

      <section className="reference-intro" aria-labelledby="page-heading">
        <div>
          <p className="eyebrow">Greenfield Worker gate</p>
          <h1 id="page-heading">Typed home controls without false certainty.</h1>
        </div>
        <div className="reference-copy">
          <p>
            This local fixture exercises renderer behavior only. It has no
            account, server connection, transport, or command authority.
          </p>
          <p className="fixture-status" role="status" aria-live="polite">
            {lastIntent
              ? `Recorded ${lastIntent}; observed values remain unchanged.`
              : "No fixture intent recorded."}
          </p>
        </div>
      </section>

      <CapabilityTable facets={facets} dispatch={dispatch} />
    </main>
  );
}
