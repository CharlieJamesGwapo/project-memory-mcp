import { useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";
import { recordTitle } from "./context";
import { validLiteralQuery } from "./sampleRetrieval";
import { taskTerms } from "./termTokenizer";
import type {
  ContextBundle,
  MemoryAdapter,
  MemoryRecord,
  Project,
  SelectionReason,
} from "./types";
export interface ContextSuggestionsProps {
  adapter: MemoryAdapter;
  session: string;
  project: Project;
  disabled: boolean;
  onApply: (records: MemoryRecord[], mode: "add" | "replace") => string | null;
  onStale: (error: unknown) => boolean;
}
const reasons: Record<SelectionReason, string> = {
  explicit_selection: "Explicit selection",
  literal_query_match: "Literal phrase match",
  ranked_term_match: "Ranked task-term match",
  latest_handoff: "Latest handoff",
  active_decision: "Active decision",
  recent_note: "Recent note",
};
const presets = [
  { key: "small", label: "Small", bytes: "8000", records: "4" },
  { key: "balanced", label: "Balanced", bytes: "16000", records: "8" },
  { key: "expanded", label: "Expanded", bytes: "32000", records: "12" },
] as const;
type PresetKey = (typeof presets)[number]["key"] | "custom";
export default function ContextSuggestions({
  adapter,
  session,
  project,
  disabled,
  onApply,
  onStale,
}: ContextSuggestionsProps) {
  const [query, setQuery] = useState("");
  const [queryMode, setQueryMode] = useState<"literal" | "terms">("literal");
  const [preset, setPreset] = useState<PresetKey>("balanced");
  const [maxBytes, setMaxBytes] = useState("16000");
  const [maxRecords, setMaxRecords] = useState("8");
  const [bundle, setBundle] = useState<ContextBundle | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [applied, setApplied] = useState("");
  const request = useRef(0);
  useEffect(() => {
    request.current++;
    setBundle(null);
    setError("");
    setApplied("");
    setBusy(false);
    setQuery("");
    setQueryMode("literal");
    return () => {
      request.current++;
    };
  }, [adapter, session, project.id]);
  function edit() {
    request.current++;
    setBundle(null);
    setBusy(false);
    setError("");
    setApplied("");
  }
  async function build(event: FormEvent) {
    event.preventDefault();
    const bytes = Number(maxBytes),
      count = Number(maxRecords);
    if (
      !Number.isInteger(bytes) ||
      bytes < 4096 ||
      bytes > 65536 ||
      !Number.isInteger(count) ||
      count < 1 ||
      count > 20
    ) {
      setError(
        "Choose a retrieval budget of 4096–65536 bytes and 1–20 records.",
      );
      return;
    }
    if (query && !validLiteralQuery(query)) {
      setError("Use an optional literal phrase of 1–2000 characters.");
      return;
    }
    if (queryMode === "terms") {
      const terms = taskTerms(query);
      if (terms.length < 1 || terms.length > 8) {
        setError(
          "Use 1–8 distinct alphanumeric task terms. Repeated ASCII terms count once.",
        );
        return;
      }
    }
    const current = ++request.current;
    setBusy(true);
    setBundle(null);
    setError("");
    setApplied("");
    try {
      const result = await adapter.query<ContextBundle>(
        session,
        "get_context_bundle",
        {
          project_id: project.id,
          ...(query ? { query } : {}),
          ...(queryMode === "terms" ? { query_mode: "terms" } : {}),
          max_bytes: bytes,
          max_records: count,
        },
      );
      if (current !== request.current) return;
      if (
        result.project.id !== project.id ||
        result.records.some((record) => record.project_id !== project.id)
      )
        throw new Error("Project mismatch");
      setBundle(result);
    } catch (cause) {
      if (current !== request.current || onStale(cause)) return;
      const code =
        typeof cause === "object" && cause !== null && "code" in cause
          ? cause.code
          : null;
      setError(
        code === "BUDGET_EXCEEDED"
          ? "The project metadata could not fit this retrieval budget. Increase the byte limit."
          : "Suggestions could not be built. Your current brief is unchanged. Try again.",
      );
    } finally {
      if (current === request.current) setBusy(false);
    }
  }
  function apply(mode: "add" | "replace") {
    if (!bundle || busy || disabled) return;
    const failure = onApply(bundle.records, mode);
    setError(failure || "");
    if (!failure)
      setApplied(
        mode === "add"
          ? "Suggestions added to your brief."
          : "Brief replaced with these suggestions.",
      );
  }
  return (
    <details className="suggestions">
      <summary>Suggest context</summary>
      <div className="suggestion-body">
        <p className="suggestion-intro">
          Find whole records for a task, or start with the latest handoff,
          active decisions, and recent notes. Review them before changing your
          brief.
        </p>
        <form noValidate onSubmit={(event) => void build(event)}>
          <fieldset className="suggestion-presets">
            <legend>Context size</legend>
            <div>
              {presets.map((option) => (
                <button
                  className="suggestion-preset"
                  type="button"
                  key={option.key}
                  aria-pressed={preset === option.key}
                  onClick={() => {
                    edit();
                    setPreset(option.key);
                    setMaxBytes(option.bytes);
                    setMaxRecords(option.records);
                  }}
                >
                  {option.label}
                </button>
              ))}
            </div>
          </fieldset>
          <p className="suggestion-explanation suggestion-size">
            {preset === "custom" ? <strong>Custom limits</strong> : null}
            {preset === "custom" ? " · " : ""}
            Up to {maxRecords || "—"} whole records ·{" "}
            {maxBytes ? Number(maxBytes).toLocaleString("en") : "—"} retrieval bytes.
          </p>
          <label className="suggestion-field">
            Task phrase <span>(optional)</span>
            <input
              aria-label="Context task phrase"
              placeholder="e.g. literal search"
              value={query}
              onChange={(event) => {
                edit();
                setQuery(event.target.value);
                if (!event.target.value) setQueryMode("literal");
              }}
            />
          </label>
          {query ? (
            <fieldset className="suggestion-match-mode">
              <legend>Match task</legend>
              <label>
                <input
                  type="radio"
                  name="context-query-mode"
                  checked={queryMode === "literal"}
                  onChange={() => {
                    edit();
                    setQueryMode("literal");
                  }}
                />
                Literal phrase
              </label>
              <label>
                <input
                  type="radio"
                  name="context-query-mode"
                  checked={queryMode === "terms"}
                  onChange={() => {
                    edit();
                    setQueryMode("terms");
                  }}
                />
                Any task terms
              </label>
            </fieldset>
          ) : null}
          {query && queryMode === "terms" ? (
            <p className="suggestion-explanation">
              Matches any of 1–8 distinct task terms. More term matches come first,
              then headline matches and recency. This is lexical, not semantic
              search.
            </p>
          ) : null}
          <details className="suggestion-advanced">
            <summary>Advanced limits</summary>
            <div className="suggestion-budgets">
              <label className="suggestion-field">
                Retrieval bytes
                <input
                  aria-label="Retrieval byte budget"
                  type="number"
                  min="4096"
                  max="65536"
                  step="1"
                  required
                  value={maxBytes}
                  onChange={(event) => {
                    edit();
                    setPreset("custom");
                    setMaxBytes(event.target.value);
                  }}
                />
              </label>
              <label className="suggestion-field">
                Record limit
                <input
                  aria-label="Suggested record limit"
                  type="number"
                  min="1"
                  max="20"
                  step="1"
                  required
                  value={maxRecords}
                  onChange={(event) => {
                    edit();
                    setPreset("custom");
                    setMaxRecords(event.target.value);
                  }}
                />
              </label>
            </div>
          </details>
          <button
            className="secondary"
            type="submit"
            disabled={busy || disabled}
          >
            {busy ? "Building suggestions…" : "Build suggestions"}
          </button>
        </form>
        <p className="suggestion-explanation">
          Retrieval bytes cover the complete MCP result, including metadata and
          its JSON text copy. Final Markdown uses a different format and has its
          own size.
        </p>
        {adapter.sample ? (
          <p className="suggestion-explanation">
            Sample budget model · synthetic records; no live archive.
          </p>
        ) : null}
        {error ? (
          <p className="status-message error" role="alert">
            {error}
          </p>
        ) : null}
        {bundle ? (
          <div className="suggestion-result" aria-live="polite">
            <p className="suggestion-measure">
              Retrieval result: {bundle.budget.used_bytes.toLocaleString("en")}{" "}
              / {bundle.budget.max_bytes.toLocaleString("en")} bytes
            </p>
            <p className="suggestion-explanation">
              Heuristic estimate:{" "}
              {bundle.budget.estimated_tokens.toLocaleString("en")} tokens
              (ceil(bytes / 4)). This is not a model token count or a measure of
              billing savings.
            </p>
            {bundle.selection.mode === "terms" ? (
              <p className="suggestion-explanation">
                Ranked by distinct task terms, then headline coverage and recency.
                Task terms: {bundle.selection.query_terms?.join(", ")}.
              </p>
            ) : null}
            <p className="suggestion-explanation">
              Candidate window: {bundle.selection.candidate_count} examined (up
              to 200)
              {bundle.selection.candidate_window_truncated
                ? " · more candidates exist"
                : "."}
            </p>
            <p className="suggestion-explanation">
              Omitted: {bundle.selection.omitted_by_budget} by byte budget;{" "}
              {bundle.selection.omitted_by_record_limit} by record limit.
              Candidates that do not fit are skipped whole so smaller records
              can follow.
            </p>
            <p className="suggestion-explanation">
              {bundle.selection.selection_complete
                ? "All candidates in this pool fit."
                : "This candidate pool is incomplete."}{" "}
              This does not describe all project history.
            </p>
            {bundle.records.length ? (
              <>
                <ol className="suggestion-records">
                  {bundle.records.map((record) => (
                    <li key={record.id}>
                      <span>{recordTitle(record)}</span>
                      <small>
                        {record.kind} ·{" "}
                        {reasons[record.selection_reason] || "Selected record"}
                        {record.matched_terms?.length
                          ? ` · matched ${record.matched_terms.join(", ")}`
                          : ""}
                      </small>
                    </li>
                  ))}
                </ol>
                <div className="suggestion-actions">
                  <button
                    className="secondary"
                    disabled={disabled || busy}
                    onClick={() => apply("add")}
                  >
                    Add suggestions
                  </button>
                  <button
                    className="text-button"
                    disabled={disabled || busy}
                    onClick={() => apply("replace")}
                  >
                    Replace brief
                  </button>
                </div>
                <p className="suggestion-explanation">
                  Your current selection changes only when you choose Add or
                  Replace.
                </p>
              </>
            ) : (
              <p className="suggestion-explanation">
                {bundle.selection.candidate_count === 0
                  ? query
                    ? "No records matched this request. Try a shorter phrase or different task terms."
                    : "No records matched this request. Current records saved to this project will appear here."
                  : bundle.selection.omitted_by_budget > 0
                    ? "Matching records were too large for this byte budget. Choose Expanded or increase the Advanced limits; records are kept whole."
                    : "No whole records are available for this request."}
              </p>
            )}
          </div>
        ) : null}
        {applied ? (
          <p className="status-message" role="status">
            {applied}
          </p>
        ) : null}
      </div>
    </details>
  );
}
