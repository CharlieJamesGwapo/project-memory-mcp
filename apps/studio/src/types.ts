export type Kind = "note" | "decision" | "handoff";
export interface ConnectionInfo {
  status: "connected" | "missing" | "error";
  path: string | null;
  default_path: string | null;
  session_id: string | null;
  error: string | null;
}
export interface Project {
  id: string;
  name: string;
  purpose: string;
  directory: string | null;
  created_at: string;
  record_count: number;
  note_count: number;
  decision_count: number;
  active_decision_count: number;
  handoff_count: number;
  last_activity: string | null;
}
export interface Page<T> {
  items: T[];
  next_cursor: string | null;
  truncated: boolean;
}
export interface Preview {
  id: string;
  project_id: string;
  kind: Kind;
  created_at: string;
  author_type: "user" | "assistant";
  title: string | null;
  preview: string;
  content_truncated: boolean;
  full_record_required: true;
  supersedes: string | null;
  superseded_by: string | null;
  next_actions?: string[];
  next_actions_truncated?: boolean;
}
interface RecordBase {
  id: string;
  project_id: string;
  kind: Kind;
  created_at: string;
  author_type: "user" | "assistant";
  sources: string[];
  superseded_by: string | null;
}
export interface Note extends RecordBase {
  kind: "note";
  title: string;
  content: string;
  review?: ReviewMetadata;
  freshness?: "unknown" | "current" | "stale";
  // Local evidence attached to a complete history result, never persisted.
  review_snapshot?: { current_head: string | null; notice: string };
}
export interface ReviewMetadata {
  format: "project-memory-review-v1";
  repository: string;
  base_commit: string;
  head_commit: string;
  summary: string;
  checks: {
    name: string;
    outcome: "passed" | "failed" | "not_run";
    result: string;
  }[];
  findings: {
    id: string;
    severity: string;
    title: string;
    path: string;
    line: number;
    evidence: string;
    correction: string;
    status: "open" | "fixed" | "accepted" | "needs_verification";
  }[];
  previous_review_id: string | null;
  revised_by?: string | null;
}
export interface ReviewPage extends Page<Note> {
  budget: { max_bytes: number; used_bytes: number };
  notice: string;
}
export interface HealthSignal {
  code: string;
  message: string;
}
export interface MemoryHealth
  extends Page<{ record_id: string; kind: Kind; signals: HealthSignal[] }> {
  project_id: string;
  summary: {
    record_count: number;
    active_decision_count: number;
    review_count: number;
    current_review_count: number;
    open_finding_count: number;
    missing_source_count: number;
    latest_handoff_id: string | null;
    suggestions: HealthSignal[];
  };
  records_examined: number;
  budget: { max_bytes: number; used_bytes: number };
  notice: string;
}
export interface SetupInput {
  database: string;
  executable: string;
  policy_path?: string;
  read_only: boolean;
}
export interface SetupReport {
  status: "ready" | "action_required";
  checks: {
    code: string;
    status: "passed" | "action_required";
    message: string;
  }[];
  configuration: Record<string, unknown> | null;
  next_actions: string[];
  notice: string;
}
export interface Decision extends RecordBase {
  kind: "decision";
  decision: string;
  rationale: string;
  supersedes: string | null;
}
export interface Handoff extends RecordBase {
  kind: "handoff";
  summary: string;
  completed: string[];
  unfinished: string[];
  next_actions: string[];
  blockers: string[];
  verification: string[];
}
export type MemoryRecord = Note | Decision | Handoff;
export interface ProjectContext {
  project: Project;
  active_decisions: { items: Preview[]; truncated: boolean };
  latest_handoff: Preview | null;
  history: Page<Preview>;
  notice: string;
}
export interface MemoryAdapter {
  sample: boolean;
  bootstrap(): Promise<ConnectionInfo>;
  chooseDatabase(): Promise<ConnectionInfo | null>;
  setupPreview(args: SetupInput): Promise<SetupReport>;
  query<T>(
    session: string,
    operation: string,
    args: Record<string, unknown>,
  ): Promise<T>;
  copy(markdown: string): Promise<void>;
  save(markdown: string): Promise<{ saved: boolean; file_name: string | null }>;
}

export type SelectionReason =
  | "explicit_selection"
  | "literal_query_match"
  | "ranked_term_match"
  | "latest_handoff"
  | "active_decision"
  | "recent_note";
export interface ContextBundle {
  project: Pick<
    Project,
    "id" | "name" | "purpose" | "directory" | "created_at"
  >;
  records: (MemoryRecord & {
    selection_reason: SelectionReason;
    matched_terms?: string[];
  })[];
  selection: {
    mode: "explicit" | "query" | "recent" | "terms";
    query_terms?: string[];
    ranking?: "term_coverage_then_headline_then_recency";
    candidate_count: number;
    candidate_window_truncated: boolean;
    omitted_by_budget: number;
    omitted_by_record_limit: number;
    selection_complete: boolean;
  };
  budget: {
    max_bytes: number;
    used_bytes: number;
    max_records: number;
    estimated_tokens: number;
    estimate_method: string;
  };
  notice: string;
}
