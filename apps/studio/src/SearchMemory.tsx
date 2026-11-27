import { useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";
import { Icon } from "./icons";
import type { Kind, MemoryAdapter, Page, Preview, Project } from "./types";
import { validLiteralQuery } from "./sampleRetrieval";
export interface SearchMemoryProps {
  adapter: MemoryAdapter;
  session: string;
  project: Project;
  onOpen: (record: Preview) => void;
  onStale: (error: unknown) => boolean;
}
export default function SearchMemory({
  adapter,
  session,
  project,
  onOpen,
  onStale,
}: SearchMemoryProps) {
  const [input, setInput] = useState("");
  const [kind, setKind] = useState<Kind | "">("");
  const [history, setHistory] = useState(false);
  const [submitted, setSubmitted] = useState<{
    query: string;
    kind: Kind | "";
    history: boolean;
  } | null>(null);
  const [items, setItems] = useState<Preview[]>([]);
  const [cursor, setCursor] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const request = useRef(0);
  useEffect(() => {
    request.current++;
    setInput("");
    setSubmitted(null);
    setItems([]);
    setCursor(null);
    setLoading(false);
    setError("");
    return () => {
      request.current++;
    };
  }, [adapter, session, project.id]);
  async function search(
    filter: NonNullable<typeof submitted>,
    nextCursor: string | null = null,
  ) {
    const current = ++request.current;
    setLoading(true);
    setError("");
    if (!nextCursor) {
      setItems([]);
      setCursor(null);
    }
    try {
      const page = await adapter.query<Page<Preview>>(
        session,
        "search_memory",
        {
          project_id: project.id,
          query: filter.query,
          ...(filter.kind ? { kind: filter.kind } : {}),
          include_superseded: filter.history,
          limit: 20,
          ...(nextCursor ? { cursor: nextCursor } : {}),
        },
      );
      if (current !== request.current) return;
      setItems((previous) =>
        nextCursor ? [...previous, ...page.items] : page.items,
      );
      setCursor(page.next_cursor);
    } catch (cause) {
      if (current === request.current && !onStale(cause))
        setError("Memory could not be searched. Try again.");
    } finally {
      if (current === request.current) setLoading(false);
    }
  }
  function submit(event: FormEvent) {
    event.preventDefault();
    if (!validLiteralQuery(input)) {
      setError("Enter a literal phrase of 1–2000 characters.");
      return;
    }
    const filter = { query: input, kind, history };
    setSubmitted(filter);
    void search(filter);
  }
  return (
    <div className="content-panel unified-search">
      <form onSubmit={submit}>
        <label className="section-label" htmlFor="memory-query">
          Search across the work
        </label>
        <div className="search-form">
          <Icon name="search" size={14} />
          <input
            id="memory-query"
            aria-label="Search all memory"
            placeholder="A phrase from a note, decision, or handoff…"
            value={input}
            onChange={(event) => setInput(event.target.value)}
          />
          <button type="submit">Search memory</button>
        </div>
        <div className="retrieval-filter-row">
          <label>
            Record kind
            <select
              aria-label="Memory record kind"
              value={kind}
              onChange={(event) => setKind(event.target.value as Kind | "")}
            >
              <option value="">All records</option>
              <option value="note">Notes</option>
              <option value="decision">Decisions</option>
              <option value="handoff">Handoffs</option>
            </select>
          </label>
          <label className="filter-check">
            <input
              type="checkbox"
              checked={history}
              onChange={(event) => setHistory(event.target.checked)}
            />
            Include superseded
          </label>
        </div>
      </form>
      <p className="search-hint">
        Literal text, including punctuation. Searches record content, never
        source pointers. Apply filters with Search memory.
      </p>
      {error ? (
        <p className="status-message error" role="alert">
          {error}
          {submitted ? (
            <button
              className="retry-inline"
              onClick={() => void search(submitted)}
            >
              Try again
            </button>
          ) : null}
        </p>
      ) : null}
      {items.length ? (
        <div className="memory-results">
          {items.map((record) => (
            <button
              className="record-list-item"
              key={record.id}
              aria-label={`Open record: ${record.title || record.preview || "Untitled record"}`}
              onClick={() => onOpen(record)}
            >
              <span className="section-label">
                {record.kind}
                {record.superseded_by ? " · superseded" : ""}
              </span>
              <h3>{record.title || record.preview || "Untitled record"}</h3>
              {record.title ? (
                <p>
                  {record.preview}
                  {record.content_truncated ? "…" : ""}
                </p>
              ) : null}
              <span className="record-meta">
                {record.created_at} · {record.author_type}
              </span>
            </button>
          ))}
        </div>
      ) : submitted && !loading && !error ? (
        <div className="empty-state">
          <h3>No records match that phrase.</h3>
          <p>Try a shorter phrase or include superseded decisions.</p>
        </div>
      ) : !submitted ? (
        <div className="empty-state">
          <h3>Follow a thought through the archive.</h3>
          <p>
            Find a phrase across notes, decision reasons, and every handoff
            section.
          </p>
        </div>
      ) : null}
      {loading ? (
        <div className="loading" role="status">
          <span className="spinner" />
          Searching project memory…
        </div>
      ) : null}
      {cursor && !loading && !error && submitted ? (
        <button
          className="secondary load-more"
          onClick={() => void search(submitted, cursor)}
        >
          Load more matches
        </button>
      ) : null}
      <p className="page-footer">
        {items.length
          ? `${items.length} matches shown${cursor ? " · more available" : ""}. `
          : ""}
        Open any result for its complete original record before adding it to a
        brief.
      </p>
    </div>
  );
}
