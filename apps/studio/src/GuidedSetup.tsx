import { useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";
import type { MemoryAdapter, SetupReport } from "./types";
export default function GuidedSetup({
  adapter,
  scope,
}: {
  adapter: MemoryAdapter;
  scope: string;
}) {
  const [open, setOpen] = useState(false);
  const [database, setDatabase] = useState("");
  const [executable, setExecutable] = useState("");
  const [policy, setPolicy] = useState("");
  const [readOnly, setReadOnly] = useState(true);
  const [report, setReport] = useState<SetupReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState("");
  const request = useRef(0);
  const invalidate = () => {
    request.current++;
    setReport(null);
    setLoading(false);
    setMessage("");
  };
  useEffect(() => {
    invalidate();
    return () => {
      request.current++;
    };
  }, [adapter, scope]);
  async function preview(event: FormEvent) {
    event.preventDefault();
    const current = ++request.current;
    setReport(null);
    setMessage("");
    setLoading(true);
    try {
      const result = await adapter.setupPreview({
        database,
        executable,
        ...(policy ? { policy_path: policy } : {}),
        read_only: readOnly,
      });
      if (current === request.current) setReport(result);
    } catch {
      if (current === request.current)
        setMessage(
          "Setup could not be previewed. Check the explicit absolute paths and try again.",
        );
    } finally {
      if (current === request.current) setLoading(false);
    }
  }
  async function copy() {
    if (!report?.configuration) return;
    const current = request.current;
    try {
      await adapter.copy(JSON.stringify(report.configuration, null, 2));
      if (current === request.current)
        setMessage("Configuration copied. Add it to your client when ready.");
    } catch {
      if (current === request.current)
        setMessage(
          "Configuration could not be copied. Select the JSON and copy it manually.",
        );
    }
  }
  return (
    <section className="guided-setup">
      <button
        className="text-button"
        aria-expanded={open}
        aria-controls="guided-setup-panel"
        onClick={() => {
          setOpen(!open);
          if (open) invalidate();
        }}
      >
        Guided setup
      </button>
      {open ? (
        <div id="guided-setup-panel" className="setup-panel">
          <div className="eyebrow">A configuration you can inspect</div>
          <h2>Connect your memory client.</h2>
          <p>
            Enter absolute paths to preview setup. Studio never runs the
            executable, creates an archive, or writes client settings. A preview
            is not a connection test.
          </p>
          <p className="overview-note">
            Studio keeps its own local read-only session. A policy preview and
            the read-only option below apply to the generated client
            configuration, not Studio’s connection.
          </p>
          {adapter.sample ? (
            <p className="synthetic-label">
              Synthetic setup preview · No real filesystem will be checked.
            </p>
          ) : null}
          <form onSubmit={(event) => void preview(event)}>
            <label>
              Database path
              <input
                aria-label="Database path"
                value={database}
                placeholder="/absolute/path/memory.sqlite"
                onChange={(e) => {
                  invalidate();
                  setDatabase(e.target.value);
                }}
                required
              />
            </label>
            <label>
              Executable path
              <input
                aria-label="Executable path"
                value={executable}
                placeholder="/absolute/path/project-memory-mcp"
                onChange={(e) => {
                  invalidate();
                  setExecutable(e.target.value);
                }}
                required
              />
            </label>
            <label>
              Policy path · optional
              <input
                aria-label="Policy path"
                value={policy}
                placeholder="/absolute/path/policy.json"
                onChange={(e) => {
                  invalidate();
                  setPolicy(e.target.value);
                }}
              />
            </label>
            <label className="filter-check">
              <input
                type="checkbox"
                checked={readOnly}
                onChange={(e) => {
                  invalidate();
                  setReadOnly(e.target.checked);
                }}
              />
              Read-only client configuration
            </label>
            <button className="primary" disabled={loading}>
              {loading ? "Previewing…" : "Preview setup"}
            </button>
          </form>
          {loading ? <p role="status">Reading setup metadata…</p> : null}
          {report ? (
            <div className="setup-report">
              <h3>
                {report.status === "ready"
                  ? "Configuration ready to inspect."
                  : "A few steps remain."}
              </h3>
              <ul className="setup-checks">
                {report.checks.map((check) => (
                  <li key={check.code}>
                    <span
                      className={`status-tag ${check.status !== "passed" ? "historical" : ""}`}
                    >
                      {check.status === "passed" ? "Passed" : "Action required"}
                    </span>
                    <p>{check.message}</p>
                  </li>
                ))}
              </ul>
              <p className="overview-note">{report.notice}</p>
              {report.next_actions.length ? (
                <ol>
                  {report.next_actions.map((action, i) => (
                    <li key={i}>{action}</li>
                  ))}
                </ol>
              ) : null}
              {report.configuration ? (
                <>
                  <pre tabIndex={0} aria-label="Client configuration JSON">
                    {JSON.stringify(report.configuration, null, 2)}
                  </pre>
                  <button className="secondary" onClick={() => void copy()}>
                    Copy configuration
                  </button>
                </>
              ) : null}
            </div>
          ) : null}
          {message ? (
            <p role="status" className="status-message">
              {message}
            </p>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}
