import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { version as appVersion } from "../package.json";
import type { FormEvent, KeyboardEvent, ReactNode } from "react";
import {
  addToContext,
  formatContext,
  MAX_RECORDS,
  recordTitle,
} from "./context";
import { isStaleSession } from "./adapter";
import { createSampleAdapter } from "./sample";
import SearchMemory from "./SearchMemory";
import ContextSuggestions from "./ContextSuggestions";
import { Brand, Icon } from "./icons";
import type { IconName } from "./icons";
import type {
  ConnectionInfo,
  MemoryAdapter,
  MemoryRecord,
  Page,
  Preview,
  Project,
  ProjectContext,
} from "./types";
type Tab = "overview" | "notes" | "decisions" | "handoffs" | "search";
const tabs: { key: Tab; name: string; icon: IconName }[] = [
  { key: "overview", name: "Overview", icon: "overview" },
  { key: "notes", name: "Notes", icon: "note" },
  { key: "decisions", name: "Decisions", icon: "clock" },
  { key: "handoffs", name: "Handoffs", icon: "stack" },
  { key: "search", name: "Search", icon: "search" },
];
function shortDate(date: string | null): string {
  if (!date) return "No activity yet";
  const value = new Date(date);
  return Number.isNaN(value.getTime())
    ? date
    : new Intl.DateTimeFormat("en", {
        day: "numeric",
        month: "short",
        year: "numeric",
        timeZone: "UTC",
      }).format(value);
}
function titleOf(record: Preview): string {
  return (
    record.title ||
    (record.kind === "note" ? "Untitled note" : record.preview) ||
    "Untitled record"
  );
}
function Loading({
  children = "Reading project memory…",
}: {
  children?: ReactNode;
}) {
  return (
    <div className="loading" role="status">
      <span className="spinner" />
      {children}
    </div>
  );
}
function Empty({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="empty-state">
      <h3>{title}</h3>
      <p>{children}</p>
    </div>
  );
}
function RecordMeta({ record }: { record: Preview }) {
  return (
    <div className="record-meta">
      <span>{shortDate(record.created_at)}</span>
      <span aria-hidden="true">·</span>
      <span>{record.author_type === "assistant" ? "Assistant" : "You"}</span>
      {record.kind === "decision" ? (
        <span
          className={`status-tag ${record.superseded_by ? "historical" : ""}`}
        >
          {record.superseded_by ? "Superseded" : "Active"}
        </span>
      ) : null}
    </div>
  );
}
function RecordButton({
  record,
  onOpen,
  compact = false,
}: {
  record: Preview;
  onOpen: (record: Preview) => void;
  compact?: boolean;
}) {
  return compact ? (
    <button
      className="recent-row"
      onClick={() => onOpen(record)}
      aria-label={`Open record: ${titleOf(record)}`}
    >
      <span className="note-icon">
        <Icon name="note" size={16} />
      </span>
      <span className="row-content">
        <h4>{titleOf(record)}</h4>
        <p>{record.preview}</p>
      </span>
      <span className="row-date">{shortDate(record.created_at)}</span>
      <Icon name="arrow" size={13} />
    </button>
  ) : (
    <button
      className="record-list-item"
      onClick={() => onOpen(record)}
      aria-label={`Open record: ${titleOf(record)}`}
    >
      <h3>{titleOf(record)}</h3>
      {record.preview !== titleOf(record) ? (
        <p>
          {record.preview}
          {record.content_truncated ? "…" : ""}
        </p>
      ) : null}
      <RecordMeta record={record} />
    </button>
  );
}
function TextSection({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  return (
    <section className="inspector-section">
      <h3>{title}</h3>
      {children}
    </section>
  );
}
function ListSection({ title, items }: { title: string; items: string[] }) {
  return (
    <TextSection title={title}>
      {items.length ? (
        <ul>
          {items.map((item, i) => (
            <li key={i}>{item}</li>
          ))}
        </ul>
      ) : (
        <p className="record-text muted">None recorded.</p>
      )}
    </TextSection>
  );
}
function Inspector({
  preview,
  record,
  loading,
  error,
  selected,
  addError,
  onClose,
  onRetry,
  onAdd,
}: {
  preview: Preview;
  record: MemoryRecord | null;
  loading: boolean;
  error: boolean;
  selected: boolean;
  addError: string;
  onClose: () => void;
  onRetry: () => void;
  onAdd: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const element = dialog.current;
    if (element && !element.open) {
      if (typeof element.showModal === "function") element.showModal();
      else element.setAttribute("open", "");
    }
    return () => {
      if (element?.open && typeof element.close === "function") element.close();
    };
  }, []);
  return (
    <dialog
      ref={dialog}
      className="record-dialog"
      aria-label="Complete memory record"
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
      onClick={(event) => {
        if (event.target === dialog.current) {
          const rect = dialog.current.getBoundingClientRect();
          if (
            event.clientX < rect.left ||
            event.clientX > rect.right ||
            event.clientY < rect.top ||
            event.clientY > rect.bottom
          )
            onClose();
        }
      }}
    >
      <div className="inspector-bar">
        <span className="section-label">Original record / {preview.kind}</span>
        <button
          className="icon-button"
          aria-label="Close record"
          onClick={onClose}
          autoFocus
        >
          <Icon name="close" />
        </button>
      </div>
      <div className="inspector-content">
        {loading ? (
          <Loading>Loading the complete record…</Loading>
        ) : error ? (
          <div className="error-state" role="alert">
            <h2 id="record-title" className="sr-only">
              Record unavailable
            </h2>
            <p>This record could not be loaded. Try again.</p>
            <button className="secondary" onClick={onRetry}>
              Try again
            </button>
          </div>
        ) : record ? (
          <>
            <div className="eyebrow">
              {record.kind === "decision" ? (
                <span
                  className={`status-tag ${record.superseded_by ? "historical" : ""}`}
                >
                  {record.superseded_by
                    ? "Superseded decision"
                    : "Active decision"}
                </span>
              ) : (
                <span>
                  {record.kind === "note"
                    ? "A note from the archive"
                    : "A recorded handoff"}
                </span>
              )}
            </div>
            <h2 id="record-title">{recordTitle(record)}</h2>
            <div className="inspector-meta">
              <p>
                Recorded by{" "}
                {record.author_type === "assistant" ? "Assistant" : "You"} ·{" "}
                <time dateTime={record.created_at}>{record.created_at}</time>
              </p>
              <p>Record {record.id}</p>
              {record.superseded_by ? (
                <p>Superseded by: {record.superseded_by}</p>
              ) : null}
              {record.kind === "decision" && record.supersedes ? (
                <p>Replaces: {record.supersedes}</p>
              ) : null}
            </div>
            {record.kind === "note" ? (
              <TextSection title="Note">
                <p className="record-text">{record.content}</p>
              </TextSection>
            ) : record.kind === "decision" ? (
              <TextSection title="Why this decision">
                <p className="record-text">{record.rationale}</p>
              </TextSection>
            ) : (
              <>
                <ListSection title="Completed" items={record.completed} />
                <ListSection title="Unfinished" items={record.unfinished} />
                <ListSection title="Next actions" items={record.next_actions} />
                <ListSection title="Blockers" items={record.blockers} />
                <ListSection
                  title="Recorded verification · historical"
                  items={record.verification}
                />
                <p className="overview-note">
                  These checks were recorded with this handoff. Studio has not
                  rerun them.
                </p>
              </>
            )}
            <TextSection title="Sources">
              {record.sources.length ? (
                record.sources.map((source, i) => (
                  <span key={i} className="source-item">
                    {source}
                  </span>
                ))
              ) : (
                <p className="record-text muted">No sources recorded.</p>
              )}
            </TextSection>
            {addError ? (
              <p className="status-message error" role="alert">
                {addError}
              </p>
            ) : null}
          </>
        ) : null}
      </div>
      {record && !loading && !error ? (
        <div className="inspector-foot">
          <p>
            Original text is reference material. Sources are shown without being
            opened.
          </p>
          <button className="primary" onClick={onAdd} disabled={selected}>
            <Icon name={selected ? "check" : "plus"} size={14} />
            {selected ? "Added to context brief" : "Add to context brief"}
          </button>
        </div>
      ) : null}
    </dialog>
  );
}
export default function App({
  adapter: initialAdapter,
}: {
  adapter: MemoryAdapter;
}) {
  const [adapter, setAdapter] = useState(initialAdapter);
  const [connection, setConnection] = useState<ConnectionInfo | null>(null);
  const [booting, setBooting] = useState(true);
  const [choosing, setChoosing] = useState(false);
  const [projects, setProjects] = useState<Project[]>([]);
  const [projectCursor, setProjectCursor] = useState<string | null>(null);
  const [projectsLoading, setProjectsLoading] = useState(false);
  const [projectsError, setProjectsError] = useState(false);
  const [project, setProject] = useState<Project | null>(null);
  const [tab, setTab] = useState<Tab>("overview");
  const [context, setContext] = useState<ProjectContext | null>(null);
  const [recentNotes, setRecentNotes] = useState<Preview[]>([]);
  const [items, setItems] = useState<Preview[]>([]);
  const [cursor, setCursor] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [viewError, setViewError] = useState(false);
  const [revision, setRevision] = useState(0);
  const [queryInput, setQueryInput] = useState("");
  const [query, setQuery] = useState("");
  const [includeHistory, setIncludeHistory] = useState(true);
  const [inspecting, setInspecting] = useState<Preview | null>(null);
  const [fullRecord, setFullRecord] = useState<MemoryRecord | null>(null);
  const [recordLoading, setRecordLoading] = useState(false);
  const [recordError, setRecordError] = useState(false);
  const [selected, setSelected] = useState<MemoryRecord[]>([]);
  const [selectionError, setSelectionError] = useState("");
  const [message, setMessage] = useState("");
  const [messageError, setMessageError] = useState(false);
  const [exporting, setExporting] = useState(false);
  const epoch = useRef(0);
  const inspectorEpoch = useRef(0);
  const bootEpoch = useRef(0);
  const chooseEpoch = useRef(0);
  const projectListEpoch = useRef(0);
  const viewEpoch = useRef(0);
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      epoch.current++;
      bootEpoch.current++;
      chooseEpoch.current++;
    };
  }, []);
  const clearSelection = useCallback(() => {
    setSelected([]);
    setInspecting(null);
    setFullRecord(null);
    setSelectionError("");
    setMessage("");
    setMessageError(false);
    setExporting(false);
    inspectorEpoch.current++;
  }, []);
  const loadConnection = useCallback(
    async (source: MemoryAdapter) => {
      const request = ++bootEpoch.current;
      epoch.current++;
      projectListEpoch.current++;
      viewEpoch.current++;
      clearSelection();
      setBooting(true);
      setProject(null);
      setProjects([]);
      setProjectCursor(null);
      setContext(null);
      setItems([]);
      try {
        const info = await source.bootstrap();
        if (!mounted.current || request !== bootEpoch.current) return;
        setConnection({ ...info });
      } catch {
        if (mounted.current && request === bootEpoch.current)
          setConnection({
            status: "error",
            path: null,
            default_path: null,
            session_id: null,
            error: "Unable to open the project archive.",
          });
      } finally {
        if (mounted.current && request === bootEpoch.current) setBooting(false);
      }
    },
    [clearSelection],
  );
  useEffect(() => {
    void loadConnection(adapter);
  }, [adapter, loadConnection]);
  const handleStale = useCallback(
    (error: unknown) => {
      if (isStaleSession(error)) {
        void loadConnection(adapter);
        return true;
      }
      return false;
    },
    [adapter, loadConnection],
  );
  useEffect(() => {
    if (connection?.status !== "connected" || !connection.session_id) return;
    const request = ++projectListEpoch.current;
    let active = true;
    setProjectsLoading(true);
    setProjectsError(false);
    adapter
      .query<Page<Project>>(connection.session_id, "studio_list_projects", {
        limit: 30,
      })
      .then((page) => {
        if (!active || request !== projectListEpoch.current) return;
        setProjects(page.items);
        setProjectCursor(page.next_cursor);
        setProject(page.items[0] ?? null);
        setTab("overview");
      })
      .catch((error) => {
        if (
          active &&
          request === projectListEpoch.current &&
          !handleStale(error)
        )
          setProjectsError(true);
      })
      .finally(() => {
        if (active && request === projectListEpoch.current)
          setProjectsLoading(false);
      });
    return () => {
      active = false;
    };
  }, [adapter, connection, handleStale]);
  const selectProject = (next: Project) => {
    if (next.id === project?.id) return;
    epoch.current++;
    viewEpoch.current++;
    clearSelection();
    setProject(next);
    setTab("overview");
    setQuery("");
    setQueryInput("");
    setContext(null);
    setItems([]);
    setCursor(null);
    setViewError(false);
  };
  const changeTab = (next: Tab) => {
    if (next === tab) return;
    viewEpoch.current++;
    setTab(next);
    setItems([]);
    setCursor(null);
    setViewError(false);
  };
  const requestView = useCallback(
    async (nextCursor: string | null = null) => {
      if (!project || !connection?.session_id) return;
      if (tab === "search") {
        setLoading(false);
        return;
      }
      const request = ++viewEpoch.current;
      const scope = epoch.current;
      setLoading(true);
      setViewError(false);
      const valid = () =>
        mounted.current &&
        scope === epoch.current &&
        request === viewEpoch.current;
      try {
        if (tab === "overview") {
          const [overview, notes] = await Promise.all([
            adapter.query<ProjectContext>(
              connection.session_id,
              "get_project_context",
              { project_id: project.id, limit: 10 },
            ),
            adapter.query<Page<Preview>>(
              connection.session_id,
              "studio_list_records",
              { project_id: project.id, kind: "note", limit: 3 },
            ),
          ]);
          if (!valid()) return;
          setContext(overview);
          setRecentNotes(notes.items);
        } else {
          const operation =
            tab === "notes" && query ? "search_notes" : "studio_list_records";
          const args: Record<string, unknown> = {
            project_id: project.id,
            limit: 20,
            ...(nextCursor ? { cursor: nextCursor } : {}),
            ...(operation === "search_notes"
              ? { query }
              : {
                  kind:
                    tab === "notes"
                      ? "note"
                      : tab === "decisions"
                        ? "decision"
                        : "handoff",
                  include_superseded: includeHistory,
                }),
          };
          const page = await adapter.query<Page<Preview>>(
            connection.session_id,
            operation,
            args,
          );
          if (!valid()) return;
          setItems((previous) =>
            nextCursor ? [...previous, ...page.items] : page.items,
          );
          setCursor(page.next_cursor);
        }
      } catch (error) {
        if (valid() && !handleStale(error)) setViewError(true);
      } finally {
        if (valid()) setLoading(false);
      }
    },
    [
      adapter,
      connection?.session_id,
      project,
      tab,
      query,
      includeHistory,
      handleStale,
    ],
  );
  useEffect(() => {
    void requestView();
    return () => {
      viewEpoch.current++;
    };
  }, [requestView, revision]);
  async function loadMoreProjects() {
    if (!projectCursor || !connection?.session_id || projectsLoading) return;
    const request = ++projectListEpoch.current;
    setProjectsLoading(true);
    setProjectsError(false);
    try {
      const page = await adapter.query<Page<Project>>(
        connection.session_id,
        "studio_list_projects",
        { limit: 30, cursor: projectCursor },
      );
      if (!mounted.current || request !== projectListEpoch.current) return;
      setProjects((previous) => [...previous, ...page.items]);
      setProjectCursor(page.next_cursor);
    } catch (error) {
      if (
        mounted.current &&
        request === projectListEpoch.current &&
        !handleStale(error)
      )
        setProjectsError(true);
    } finally {
      if (mounted.current && request === projectListEpoch.current)
        setProjectsLoading(false);
    }
  }
  async function chooseDatabase() {
    if (choosing) return;
    setChoosing(true);
    const request = ++chooseEpoch.current;
    setMessage("");
    setMessageError(false);
    try {
      const info = await initialAdapter.chooseDatabase();
      if (!mounted.current || request !== chooseEpoch.current) return;
      if (!info) {
        setMessage("Open canceled. Your current workspace is unchanged.");
        return;
      }
      if (info.status !== "connected") {
        setMessage(
          "That archive could not be opened. Choose an existing Project Memory database.",
        );
        setMessageError(true);
        return;
      }
      // A picker owns its lifecycle independently of session reconnects.
      // Its successful connection supersedes any older bootstrap response.
      bootEpoch.current++;
      setBooting(false);
      epoch.current++;
      projectListEpoch.current++;
      viewEpoch.current++;
      clearSelection();
      setProject(null);
      setProjects([]);
      setProjectCursor(null);
      setContext(null);
      setItems([]);
      if (adapter !== initialAdapter) setAdapter(initialAdapter);
      else setConnection({ ...info });
    } catch {
      if (mounted.current && request === chooseEpoch.current) {
        setMessage(
          "That archive could not be opened. Choose an existing Project Memory database.",
        );
        setMessageError(true);
      }
    } finally {
      if (mounted.current && request === chooseEpoch.current)
        setChoosing(false);
    }
  }
  async function openRecord(preview: Preview) {
    if (!project || !connection?.session_id) return;
    const request = ++inspectorEpoch.current;
    const scope = epoch.current;
    setInspecting(preview);
    setFullRecord(null);
    setRecordLoading(true);
    setRecordError(false);
    setSelectionError("");
    try {
      const record = await adapter.query<MemoryRecord>(
        connection.session_id,
        "get_record",
        { project_id: project.id, record_id: preview.id },
      );
      if (
        !mounted.current ||
        scope !== epoch.current ||
        request !== inspectorEpoch.current
      )
        return;
      if (record.project_id !== project.id || record.id !== preview.id)
        throw new Error("Mismatched record.");
      setFullRecord(record);
    } catch (error) {
      if (
        mounted.current &&
        scope === epoch.current &&
        request === inspectorEpoch.current &&
        !handleStale(error)
      )
        setRecordError(true);
    } finally {
      if (
        mounted.current &&
        scope === epoch.current &&
        request === inspectorEpoch.current
      )
        setRecordLoading(false);
    }
  }
  function closeRecord() {
    inspectorEpoch.current++;
    setInspecting(null);
    setFullRecord(null);
  }
  function addRecord() {
    if (!project || !fullRecord || recordLoading || recordError) return;
    try {
      const next = addToContext(project, selected, fullRecord);
      setSelected(next);
      setSelectionError("");
      setMessage("");
    } catch (error) {
      setSelectionError(
        error instanceof Error
          ? error.message
          : "This record could not be added.",
      );
    }
  }
  function applySuggestions(
    records: MemoryRecord[],
    mode: "add" | "replace",
  ): string | null {
    if (!project || exporting || choosing)
      return "Wait for the current action before changing your brief.";
    try {
      const next = records.reduce(
        (selection, record) => addToContext(project, selection, record),
        mode === "replace" ? [] : selected,
      );
      setSelected(next);
      setMessage("");
      setMessageError(false);
      return null;
    } catch (error) {
      return error instanceof Error
        ? error.message
        : "These records could not be added. Your brief is unchanged.";
    }
  }
  const markdown = useMemo(
    () => (project && selected.length ? formatContext(project, selected) : ""),
    [project, selected],
  );
  const bytes = useMemo(
    () => new TextEncoder().encode(markdown).byteLength,
    [markdown],
  );
  async function exportBrief(action: "copy" | "save") {
    if (!markdown || exporting) return;
    const scope = epoch.current;
    setExporting(true);
    setMessage("");
    setMessageError(false);
    try {
      if (action === "copy") {
        await adapter.copy(markdown);
        if (mounted.current && scope === epoch.current)
          setMessage("Copied. Your next conversation is ready.");
      } else {
        const result = await adapter.save(markdown);
        if (mounted.current && scope === epoch.current)
          setMessage(
            result.saved
              ? initialAdapter.sample
                ? "Markdown download started."
                : `Saved${result.file_name ? ` as ${result.file_name}` : ""}.`
              : "Save canceled. Your brief is still here.",
          );
      }
    } catch {
      if (mounted.current && scope === epoch.current) {
        setMessage(
          action === "copy"
            ? "Copy failed. Try saving your brief as Markdown."
            : "Your brief could not be saved. Please try again.",
        );
        setMessageError(true);
      }
    } finally {
      if (mounted.current && scope === epoch.current) setExporting(false);
    }
  }
  function submitSearch(event: FormEvent) {
    event.preventDefault();
    viewEpoch.current++;
    setItems([]);
    setCursor(null);
    setQuery(queryInput.trim());
    if (queryInput.trim() === query) setRevision((value) => value + 1);
  }
  function tabKeys(event: KeyboardEvent<HTMLButtonElement>, index: number) {
    let next = index;
    if (event.key === "ArrowRight") next = (index + 1) % tabs.length;
    else if (event.key === "ArrowLeft")
      next = (index + tabs.length - 1) % tabs.length;
    else if (event.key === "Home") next = 0;
    else if (event.key === "End") next = tabs.length - 1;
    else return;
    event.preventDefault();
    changeTab(tabs[next].key);
    document.getElementById(`tab-${tabs[next].key}`)?.focus();
  }
  const nativeAvailable = !initialAdapter.sample;
  if (booting)
    return (
      <div className="welcome">
        <div>
          <Brand />
          <Loading>Opening your project archive…</Loading>
        </div>
      </div>
    );
  if (connection?.status !== "connected")
    return (
      <main className="welcome">
        <div className="welcome-card">
          <Brand />
          <div className="eyebrow">
            <span className="tiny-line" />
            Local knowledge, carried forward
          </div>
          <h1>A place for project memory.</h1>
          <p>
            See what you learned, why you decided, and where to pick up next.
            Open an existing Project Memory archive to begin.
          </p>
          {connection?.status === "error" ? (
            <p className="status-message error" role="alert">
              Your archive could not be opened. Choose an existing database or
              explore the sample workspace.
            </p>
          ) : null}
          <div className="welcome-actions">
            <button
              className="primary"
              onClick={() => void chooseDatabase()}
              disabled={choosing}
            >
              <Icon name="folder" />
              {choosing ? "Opening…" : "Open Database"}
            </button>
            <button
              className="secondary"
              onClick={() => {
                chooseEpoch.current++;
                setChoosing(false);
                const sample = createSampleAdapter();
                setAdapter(
                  initialAdapter.sample
                    ? sample
                    : {
                        ...sample,
                        copy: initialAdapter.copy,
                        save: initialAdapter.save,
                      },
                );
              }}
            >
              Explore sample workspace
            </button>
          </div>
          <details className="welcome-setup">
            <summary>Need to create your first archive?</summary>
            <p>
              With the Project Memory server on your PATH, run this command in
              your terminal, then open the database path it prints:
            </p>
            <code>project-memory-mcp init</code>
            <p>
              If you downloaded the server without installing it on PATH, use
              its full executable path followed by <code>init</code>. The archive
              starts empty. Connect your MCP client to add projects and memory;
              Studio opens those records read-only.
            </p>
          </details>
          {message ? (
            <p
              className={`status-message ${messageError ? "error" : ""}`}
              role="status"
            >
              {message}
            </p>
          ) : null}
          {connection?.default_path ? (
            <p className="connection-path">
              Default archive: {connection.default_path}
            </p>
          ) : null}
          <p className="switch-line">
            Existing archives are opened read-only. Your memories stay on this
            computer.
          </p>
        </div>
      </main>
    );
  return (
    <div className="shell">
      <aside className="sidebar" aria-label="Project navigation">
        <Brand />
        <div className="sidebar-heading">
          <span>Your projects</span>
          <span className="count-label">
            {projects.length}
            {projectCursor ? "+" : ""}
          </span>
        </div>
        <div className="project-list">
          <nav className="project-nav" aria-label="Projects">
            {projects.map((item) => (
              <button
                key={item.id}
                className={`project-button ${project?.id === item.id ? "active" : ""}`}
                aria-label={`Open project: ${item.name}`}
                aria-current={project?.id === item.id ? "page" : undefined}
                title={`${item.directory ?? "No directory"} · ${item.id}`}
                onClick={() => selectProject(item)}
              >
                <span className="project-dot" />
                <span>
                  <span className="project-name">{item.name}</span>
                  <span className="project-meta">
                    {item.record_count}{" "}
                    {item.record_count === 1 ? "record" : "records"}
                  </span>
                  <span className="project-id">{item.id.slice(-8)}</span>
                </span>
              </button>
            ))}
          </nav>
          {projectCursor ? (
            <button
              className="text-button"
              style={{ margin: "18px 10px" }}
              disabled={projectsLoading}
              onClick={() => void loadMoreProjects()}
            >
              Load more projects
            </button>
          ) : null}
          {projectsLoading ? <Loading>Loading…</Loading> : null}
          {projectsError ? (
            <div className="status-message error" role="alert">
              Projects could not be loaded.
              <button
                className="retry-inline"
                onClick={() =>
                  projectCursor
                    ? void loadMoreProjects()
                    : void loadConnection(adapter)
                }
              >
                Try again
              </button>
            </div>
          ) : null}
        </div>
        <div className="sidebar-foot">
          <div className="connection-label">
            <span className="connection-dot" />
            {adapter.sample ? "Sample workspace" : "Local archive connected"}
          </div>
          <p className="connection-path" title={connection.path ?? undefined}>
            {adapter.sample
              ? "Synthetic records for a closer look. No live database is connected."
              : connection.path}
          </p>
          {nativeAvailable ? (
            <button
              className="text-button"
              disabled={choosing}
              onClick={() => void chooseDatabase()}
            >
              <Icon name="folder" size={12} />
              {choosing ? "Choosing archive…" : "Open database"}
            </button>
          ) : (
            <span className="connection-path">
              Open a real archive in the desktop app.
            </span>
          )}
        </div>
        <div className="sidebar-bottom">
          <span>Read-only by design</span>
          <span>v{appVersion}</span>
        </div>
      </aside>
      <main className="workspace">
        <div className="topbar">
          <div className="breadcrumb">
            <span>Workspace</span>
            <span aria-hidden="true">/</span>
            <strong>Project memory</strong>
          </div>
          <div className="topbar-actions">
            <span className="mode-label">
              {adapter.sample ? "Sample workspace" : "Local · read-only"}
            </span>
            <button
              className="refresh-button"
              disabled={choosing || projectsLoading || exporting}
              onClick={() => void loadConnection(adapter)}
            >
              <Icon name="refresh" size={12} />
              Refresh
            </button>
          </div>
        </div>
        {project ? (
          <>
            <header className="project-header">
              <div className="eyebrow">
                <span className="tiny-line" />A living record of the work
              </div>
              <h1>{project.name}</h1>
              <p className="project-description">
                {project.purpose || "No project purpose has been recorded yet."}
              </p>
              <div className="project-stats">
                <span>
                  <strong>{project.note_count}</strong> notes
                </span>
                <span>
                  <strong>{project.active_decision_count}</strong> active
                  decisions
                </span>
                <span>
                  <strong>{project.handoff_count}</strong> handoffs
                </span>
                <span>Last activity {shortDate(project.last_activity)}</span>
              </div>
            </header>
            <div className="tabs" role="tablist" aria-label="Project views">
              {tabs.map((item, index) => (
                <button
                  id={`tab-${item.key}`}
                  key={item.key}
                  className="tab"
                  role="tab"
                  aria-label={item.name}
                  aria-selected={tab === item.key}
                  aria-controls="project-panel"
                  tabIndex={tab === item.key ? 0 : -1}
                  onClick={() => changeTab(item.key)}
                  onKeyDown={(event) => tabKeys(event, index)}
                >
                  <Icon name={item.icon} size={13} />
                  {item.name}
                  {item.key !== "overview" && item.key !== "search" ? (
                    <span className="tab-count">
                      {item.key === "notes"
                        ? project.note_count
                        : item.key === "decisions"
                          ? project.decision_count
                          : project.handoff_count}
                    </span>
                  ) : null}
                </button>
              ))}
            </div>
            <section
              id="project-panel"
              role="tabpanel"
              aria-labelledby={`tab-${tab}`}
              tabIndex={0}
            >
              {tab === "overview" ? (
                loading ? (
                  <Loading />
                ) : viewError ? (
                  <ViewError retry={() => setRevision((v) => v + 1)} />
                ) : context ? (
                  <>
                    <div className="overview-grid">
                      <section>
                        <div className="section-heading">
                          <span className="section-label">
                            Where we left off
                          </span>
                          <Icon name="stack" size={13} />
                        </div>
                        {context.latest_handoff ? (
                          <div className="handoff-card">
                            <div className="eyebrow">
                              Latest recorded handoff
                            </div>
                            <h3>{titleOf(context.latest_handoff)}</h3>
                            {context.latest_handoff.preview !==
                            titleOf(context.latest_handoff) ? (
                              <p>{context.latest_handoff.preview}</p>
                            ) : null}
                            {context.latest_handoff.next_actions?.length ? (
                              <>
                                <div className="next-label">Next up</div>
                                <p className="next-action">
                                  {context.latest_handoff.next_actions[0]}
                                </p>
                              </>
                            ) : null}
                            <div className="record-date">
                              {shortDate(context.latest_handoff.created_at)} ·{" "}
                              {context.latest_handoff.author_type ===
                              "assistant"
                                ? "Assistant"
                                : "You"}
                            </div>
                            <button
                              onClick={() =>
                                void openRecord(context.latest_handoff!)
                              }
                              aria-label={`Open record: ${titleOf(context.latest_handoff)}`}
                            >
                              Read full handoff
                              <Icon name="arrow" size={14} />
                            </button>
                          </div>
                        ) : (
                          <Empty title="A fresh starting point">
                            No handoffs have been recorded for this project yet.
                          </Empty>
                        )}
                      </section>
                      <section>
                        <div className="section-heading">
                          <span className="section-label">
                            Decisions to carry forward
                          </span>
                          <span className="activity-count">
                            {project.active_decision_count
                              .toString()
                              .padStart(2, "0")}
                          </span>
                        </div>
                        <div className="decision-list">
                          {context.active_decisions.items.length ? (
                            context.active_decisions.items
                              .slice(0, 4)
                              .map((decision, i) => (
                                <button
                                  className="decision-row"
                                  key={decision.id}
                                  onClick={() => void openRecord(decision)}
                                  aria-label={`Open record: ${titleOf(decision)}`}
                                >
                                  <span className="decision-number">
                                    {String(i + 1).padStart(2, "0")}
                                  </span>
                                  <span>
                                    <span className="decision-title">
                                      {titleOf(decision)}
                                    </span>
                                    <RecordMeta record={decision} />
                                  </span>
                                </button>
                              ))
                          ) : (
                            <Empty title="Room for decisions">
                              Settled decisions will appear here.
                            </Empty>
                          )}
                        </div>
                        {project.active_decision_count > 4 ||
                        context.active_decisions.truncated ? (
                          <button
                            className="text-button"
                            style={{ marginTop: 17 }}
                            onClick={() => changeTab("decisions")}
                          >
                            View all decisions
                            <Icon name="arrow" size={12} />
                          </button>
                        ) : null}
                      </section>
                    </div>
                    <section className="recent-section">
                      <div className="section-heading">
                        <span className="section-label">
                          Notes from the work
                        </span>
                        <button
                          className="text-button"
                          onClick={() => changeTab("notes")}
                        >
                          View all notes
                          <Icon name="arrow" size={12} />
                        </button>
                      </div>
                      {recentNotes.length ? (
                        recentNotes.map((note) => (
                          <RecordButton
                            key={note.id}
                            record={note}
                            onOpen={(p) => void openRecord(p)}
                            compact
                          />
                        ))
                      ) : (
                        <Empty title="The notebook is open">
                          There are no notes in this project yet.
                        </Empty>
                      )}
                    </section>
                    <div className="reference-note">
                      <Icon name="shield" size={13} />
                      <span>
                        A record of what was known at the time. Open any item
                        for its complete text, original sources, and date.
                      </span>
                    </div>
                  </>
                ) : null
              ) : tab === "search" ? (
                <SearchMemory
                  key={`${connection.session_id}:${project.id}`}
                  adapter={adapter}
                  session={connection.session_id!}
                  project={project}
                  onOpen={(record) => void openRecord(record)}
                  onStale={handleStale}
                />
              ) : (
                <div className="content-panel">
                  <div className="list-toolbar">
                    {tab === "notes" ? (
                      <form className="search-form" onSubmit={submitSearch}>
                        <Icon name="search" size={14} />
                        <input
                          aria-label="Search notes"
                          placeholder="Find a thought, phrase, or detail…"
                          value={queryInput}
                          onChange={(event) =>
                            setQueryInput(event.target.value)
                          }
                        />
                        {queryInput ? (
                          <button
                            type="button"
                            aria-label="Clear search"
                            onClick={() => {
                              setQueryInput("");
                              setQuery("");
                              setRevision((v) => v + 1);
                            }}
                          >
                            <Icon name="close" size={12} />
                          </button>
                        ) : null}
                        <button type="submit">Search</button>
                      </form>
                    ) : (
                      <h2 className="toolbar-title">
                        {tab === "decisions"
                          ? "The decisions behind the work."
                          : "A thread to pick up."}
                      </h2>
                    )}
                    {tab === "decisions" ? (
                      <label className="filter-check">
                        <input
                          type="checkbox"
                          checked={includeHistory}
                          onChange={(event) => {
                            setIncludeHistory(event.target.checked);
                            setItems([]);
                            setCursor(null);
                          }}
                        />
                        Include superseded
                      </label>
                    ) : null}
                  </div>
                  {tab === "notes" ? (
                    <p className="search-hint">
                      Literal phrase search across note titles and content.
                      Punctuation stays literal.
                    </p>
                  ) : null}
                  {viewError ? (
                    <ViewError retry={() => setRevision((v) => v + 1)} />
                  ) : null}
                  {items.length ? (
                    <div>
                      {items.map((item) =>
                        tab === "decisions" ? (
                          <div
                            key={item.id}
                            className={`timeline-row ${item.superseded_by ? "historical" : ""}`}
                          >
                            <RecordButton
                              record={item}
                              onOpen={(p) => void openRecord(p)}
                            />
                          </div>
                        ) : (
                          <RecordButton
                            key={item.id}
                            record={item}
                            onOpen={(p) => void openRecord(p)}
                          />
                        ),
                      )}
                    </div>
                  ) : !loading && !viewError ? (
                    <Empty
                      title={
                        query
                          ? "No notes match that phrase."
                          : tab === "notes"
                            ? "A little room to think."
                            : tab === "decisions"
                              ? "No decisions recorded yet."
                              : "No handoffs recorded yet."
                      }
                    >
                      {query
                        ? "Try a shorter phrase or clear the search to browse all notes."
                        : "Records saved through your Project Memory workflow will appear here after a refresh."}
                    </Empty>
                  ) : null}
                  {loading ? <Loading /> : null}
                  {cursor && !loading && !viewError ? (
                    <button
                      className="secondary load-more"
                      onClick={() => void requestView(cursor)}
                    >
                      Load more {tab}
                    </button>
                  ) : null}
                  <p className="page-footer">
                    {items.length
                      ? `${items.length} ${cursor ? "records shown · more available" : "records shown"}. `
                      : ""}
                    {tab === "decisions"
                      ? "Superseded decisions preserve the reasoning that came before."
                      : tab === "handoffs"
                        ? "Verification belongs to the original handoff date. Studio has not rerun recorded checks."
                        : "Open a note to read and select the complete record."}
                  </p>
                </div>
              )}
            </section>
          </>
        ) : projectsLoading ? (
          <Loading>Opening your projects…</Loading>
        ) : !projectsError ? (
          <Empty title="Every project begins somewhere.">
            This archive has no projects yet. Add one through your Project
            Memory workflow, then refresh.
          </Empty>
        ) : null}
        {adapter.sample ? (
          <p className="sample-banner">
            Sample workspace · These projects and records are synthetic. No live
            database is connected.
          </p>
        ) : null}
      </main>
      <aside className="brief" aria-label="Context brief">
        <div className="brief-kicker">
          <Icon name="stack" size={13} />
          Carry the useful things
        </div>
        <div className="brief-head">
          <h2>Context brief</h2>
          <span className="count-label">
            {selected.length.toString().padStart(2, "0")}
          </span>
        </div>
        <p className="brief-description">
          A considered selection for your next conversation. Original words,
          with their sources intact.
        </p>
        <div className="brief-content">
          {project && connection.session_id ? (
            <ContextSuggestions
              key={`${connection.session_id}:${project.id}`}
              adapter={adapter}
              session={connection.session_id}
              project={project}
              disabled={exporting || choosing}
              onApply={applySuggestions}
              onStale={handleStale}
            />
          ) : null}
          {selected.length ? (
            selected.map((record) => (
              <div className="brief-card" key={record.id}>
                <div className="brief-card-main">
                  <span className="section-label">
                    {record.kind}
                    {record.superseded_by ? " · superseded" : ""}
                  </span>
                  <h4>{recordTitle(record)}</h4>
                  <div className="record-meta">
                    {shortDate(record.created_at)}
                  </div>
                </div>
                <button
                  className="icon-button"
                  aria-label={`Remove ${recordTitle(record)} from context`}
                  disabled={exporting}
                  onClick={() => {
                    setSelected((previous) =>
                      previous.filter((item) => item.id !== record.id),
                    );
                    setMessage("");
                  }}
                >
                  <Icon name="close" size={12} />
                </button>
              </div>
            ))
          ) : (
            <div className="brief-empty">
              <span className="brief-empty-art">
                <Icon name="plus" size={20} />
              </span>
              <h3>Your next conversation starts here.</h3>
              <p>Open a note, decision, or handoff and add it to your brief.</p>
            </div>
          )}
        </div>
        <div className="brief-foot">
          <p className="brief-size-label">Final Markdown</p>
          <div className="brief-limit">
            <span>
              {selected.length} / {MAX_RECORDS} records
            </span>
            <span>
              {bytes < 1024 ? `${bytes} B` : `${(bytes / 1024).toFixed(1)} KiB`}{" "}
              / 2 MiB
            </span>
          </div>
          <p className="brief-estimate">
            Markdown estimate: {Math.ceil(bytes / 4).toLocaleString("en")}{" "}
            tokens (bytes / 4, rounded up; heuristic, not model usage).
          </p>
          <button
            className="primary"
            disabled={!selected.length || exporting}
            onClick={() => void exportBrief("copy")}
          >
            <Icon name="copy" size={13} />
            {exporting ? "Working…" : "Copy Markdown"}
          </button>
          <button
            className="secondary"
            disabled={!selected.length || exporting}
            onClick={() => void exportBrief("save")}
          >
            <Icon name="download" size={13} />
            Save Markdown
          </button>
          {message ? (
            <p
              className={`status-message ${messageError ? "error" : ""}`}
              role="status"
            >
              {message}
            </p>
          ) : null}
          <p className="brief-footer-note">
            Selected full records only.
            <br />
            Nothing leaves until you copy or save.
          </p>
        </div>
      </aside>
      {inspecting ? (
        <Inspector
          preview={inspecting}
          record={fullRecord}
          loading={recordLoading}
          error={recordError}
          selected={
            !!fullRecord &&
            selected.some((record) => record.id === fullRecord.id)
          }
          addError={selectionError}
          onClose={closeRecord}
          onRetry={() => void openRecord(inspecting)}
          onAdd={addRecord}
        />
      ) : null}
    </div>
  );
}
function ViewError({ retry }: { retry: () => void }) {
  return (
    <div className="error-state" role="alert">
      <p>
        These records could not be loaded. Your archive has not been changed.
      </p>
      <button className="secondary" onClick={retry}>
        Try again
      </button>
    </div>
  );
}
