import { useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";
import type {
  MemoryAdapter,
  MemoryHealth,
  Note,
  Preview,
  Project,
  ReviewPage,
} from "./types";
export function validHead(head: string): boolean {
  return (
    head === "" || ([40, 64].includes(head.length) && /^[0-9a-f]+$/.test(head))
  );
}
function safeError(cause: unknown): string {
  const code =
    typeof cause === "object" && cause !== null && "code" in cause
      ? cause.code
      : "";
  if (code === "BUDGET_EXCEEDED")
    return "A complete result did not fit the retrieval budget. Open a known record through Search to inspect its complete note.";
  if (code === "INVALID_CURSOR" || code === "VALIDATION_ERROR")
    return "This page can no longer be used. Load a fresh first page with a valid full commit ID.";
  return "Evidence could not be loaded. Try a fresh first page.";
}
function preview(note: Note): Preview {
  return {
    ...note,
    title: note.title,
    preview: note.content,
    content_truncated: false,
    full_record_required: true,
    supersedes: null,
  };
}
export default function EvidenceViews({
  mode,
  adapter,
  session,
  project,
  onOpen,
  onStale,
}: {
  mode: "reviews" | "health";
  adapter: MemoryAdapter;
  session: string;
  project: Project;
  onOpen: (record: Preview, complete?: Note, sourceSession?: string) => void;
  onStale: (error: unknown) => boolean;
}) {
  const [head, setHead] = useState("");
  const [result, setResult] = useState<ReviewPage | MemoryHealth | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const request = useRef(0);
  const invalidate = () => {
    request.current++;
    setResult(null);
    setLoading(false);
    setError("");
  };
  useEffect(() => {
    invalidate();
    setHead("");
    return () => {
      request.current++;
    };
  }, [adapter, session, project.id, mode]);
  async function load(cursor: string | null = null) {
    const current = ++request.current;
    if (!validHead(head)) {
      setResult(null);
      setError(
        "Enter a full lowercase hexadecimal commit ID: 40 or 64 characters, or leave it blank.",
      );
      return;
    }
    setLoading(true);
    setError("");
    if (!cursor) setResult(null);
    try {
      const value = await adapter.query<ReviewPage | MemoryHealth>(
        session,
        mode === "reviews" ? "get_review_history" : "get_memory_health",
        {
          project_id: project.id,
          ...(head ? { current_head: head } : {}),
          limit: 20,
          max_bytes: 16000,
          ...(cursor ? { cursor } : {}),
        },
      );
      if (current !== request.current) return;
      if (
        mode === "health" &&
        (value as MemoryHealth).project_id !== project.id
      )
        throw new Error("Mismatched project.");
      if (
        mode === "reviews" &&
        (value as ReviewPage).items.some(
          (note) => note.project_id !== project.id || !note.review,
        )
      )
        throw new Error("Mismatched review.");
      setResult((previous) =>
        cursor && previous
          ? ({ ...value, items: [...previous.items, ...value.items] } as
              | ReviewPage
              | MemoryHealth)
          : value,
      );
    } catch (cause) {
      if (current === request.current && !onStale(cause))
        setError(safeError(cause));
    } finally {
      if (current === request.current) setLoading(false);
    }
  }
  const reviews = mode === "reviews" ? (result as ReviewPage | null) : null;
  const health = mode === "health" ? (result as MemoryHealth | null) : null;
  return (
    <div className="content-panel evidence-view">
      <div className="eyebrow">
        {mode === "reviews"
          ? "Supplied evidence, kept complete"
          : "Small signals for the next pass"}
      </div>
      <h2>
        {mode === "reviews"
          ? "Reviews at a recorded moment."
          : "Make room for a memory check."}
      </h2>
      <p className="search-hint">
        {mode === "reviews"
          ? "Freshness compares only the commit ID you enter. A matching head does not rerun checks or verify a finding."
          : "A bounded inspection of saved metadata and literal text. Suggestions do not establish semantic conflicts, source validity, or archive safety."}
      </p>
      <form
        className="evidence-form"
        onSubmit={(e: FormEvent) => {
          e.preventDefault();
          void load();
        }}
      >
        <label htmlFor={`head-${mode}`}>
          Current head · optional
          <input
            id={`head-${mode}`}
            aria-label={`Current head for ${mode}`}
            value={head}
            placeholder="Full 40- or 64-character lowercase commit ID"
            autoCapitalize="none"
            spellCheck={false}
            onChange={(e) => {
              invalidate();
              setHead(e.target.value);
            }}
          />
        </label>
        <button className="primary" disabled={loading}>
          {mode === "reviews" ? "Load reviews" : "Inspect health"}
        </button>
      </form>
      {adapter.sample ? (
        <p className="synthetic-label">
          Synthetic {mode} · No live archive or repository is inspected.
        </p>
      ) : null}
      {error ? (
        <p className="status-message error" role="alert">
          {error}
          <button className="retry-inline" onClick={() => void load()}>
            Start again
          </button>
        </p>
      ) : null}
      {reviews ? (
        <div className="review-list">
          {reviews.items.length ? (
            reviews.items.map((note) => (
              <article className="review-card" key={note.id}>
                <div className="record-meta">
                  <span>{note.created_at}</span>
                  <span>{note.author_type}</span>
                  <span className="status-tag historical">
                    {note.freshness === "current"
                      ? "Current supplied head"
                      : note.freshness === "stale"
                        ? "Historical head"
                        : "Head not supplied"}
                  </span>
                </div>
                <button
                  className="review-open"
                  aria-label={`Open record: ${note.title}`}
                  onClick={() =>
                    onOpen(
                      preview(note),
                      {
                        ...note,
                        review_snapshot: {
                          current_head: head || null,
                          notice: reviews.notice,
                        },
                      },
                      session,
                    )
                  }
                >
                  <h3>{note.title}</h3>
                </button>
                <p>
                  {note.review!.repository} · {note.review!.findings.length}{" "}
                  findings · {note.review!.checks.length} checks
                </p>
                <p className="commit-line">
                  Base <code>{note.review!.base_commit}</code>
                  <br />
                  Head <code>{note.review!.head_commit}</code>
                </p>
                <p className="overview-note">
                  {note.review!.previous_review_id
                    ? `Revises ${note.review!.previous_review_id}`
                    : "First review"}
                  {note.review!.revised_by
                    ? ` · Revised by ${note.review!.revised_by}`
                    : " · No revision in this snapshot"}
                </p>
                <p className="overview-note">
                  Open the complete note for findings, checks, sources, and
                  explicit Add.
                </p>
              </article>
            ))
          ) : (
            <div className="empty-state">
              <h3>No reviews in this snapshot.</h3>
              <p>
                Save reviews through your memory client, then load a fresh page.
              </p>
            </div>
          )}
        </div>
      ) : null}
      {health ? (
        <>
          <dl className="health-summary">
            {[
              ["Records", health.summary.record_count],
              ["Active decisions", health.summary.active_decision_count],
              ["Reviews", health.summary.review_count],
              ["Unrevised reviews", health.summary.current_review_count],
              [
                "Open or unverified findings",
                health.summary.open_finding_count,
              ],
              ["Without sources", health.summary.missing_source_count],
            ].map(([label, value]) => (
              <div key={label}>
                <dt>{label}</dt>
                <dd>{value}</dd>
              </div>
            ))}
          </dl>
          <p className="overview-note">
            Latest handoff:{" "}
            {health.summary.latest_handoff_id ?? "None recorded"}
          </p>
          {health.summary.suggestions.length ? (
            <ul className="health-suggestions">
              {health.summary.suggestions.map((s, i) => (
                <li key={i}>
                  <strong>{s.code.replaceAll("_", " ")}</strong>
                  <p>{s.message}</p>
                </li>
              ))}
            </ul>
          ) : (
            <p>
              No summary suggestions in this snapshot. This is not a verdict on
              the whole archive.
            </p>
          )}
          <p className="page-footer">
            Records examined on this page: {health.records_examined}.{" "}
            {health.items.length} assessments shown
            {health.next_cursor ? " · more remain" : ""}.
          </p>
          <div className="health-items">
            {health.items.map((item) => (
              <article className="health-item" key={item.record_id}>
                <div className="section-label">{item.kind}</div>
                <code>{item.record_id}</code>
                {item.signals.length ? (
                  <ul>
                    {item.signals.map((signal, i) => (
                      <li key={i}>
                        <strong>{signal.code.replaceAll("_", " ")}</strong>
                        <p>{signal.message}</p>
                      </li>
                    ))}
                  </ul>
                ) : (
                  <p className="overview-note">
                    No signals for this record in this inspection.
                  </p>
                )}
              </article>
            ))}
          </div>
        </>
      ) : null}
      {loading ? (
        <p className="loading" role="status">
          <span className="spinner" />
          Reading {mode}…
        </p>
      ) : null}
      {result ? (
        <>
          <p className="page-footer">
            Page budget: {result.budget.used_bytes.toLocaleString()} /{" "}
            {result.budget.max_bytes.toLocaleString()} canonical response bytes.{" "}
            {result.truncated
              ? "This result is incomplete; follow the next page."
              : "End of this snapshot."}
          </p>
          <p className="overview-note">{result.notice}</p>
          {result.next_cursor && !loading && !error ? (
            <button
              className="secondary load-more"
              onClick={() => void load(result.next_cursor)}
            >
              Load more {mode === "reviews" ? "reviews" : "assessments"}
            </button>
          ) : null}
        </>
      ) : null}
    </div>
  );
}
