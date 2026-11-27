import type {
  ConnectionInfo,
  MemoryAdapter,
  MemoryRecord,
  Page,
  Preview,
  Project,
  ProjectContext,
} from "./types";
import { formatContext } from "./context";
import { sampleHealth, sampleReviews, sampleSetup } from "./sampleGuided";
import {
  matchesHumanText,
  sampleContextBundle,
  validLiteralQuery,
} from "./sampleRetrieval";
const projectIds = [
  "2c82826a-2ed0-4be2-93e2-0a87ad106501",
  "2c82826a-2ed0-4be2-93e2-0a87ad106502",
  "2c82826a-2ed0-4be2-93e2-0a87ad106503",
];
const rid = (n: number) =>
  `f6181db9-c091-49a5-a658-${String(n).padStart(12, "0")}`;
const common = (n: number, date: string, project = 0) => ({
  id: rid(n),
  project_id: projectIds[project],
  created_at: `2026-11-${date}T09:30:00Z`,
  author_type: "assistant" as const,
  sources: ["docs/field-notes.md"],
  superseded_by: null,
});
const records: MemoryRecord[] = [
  {
    ...common(1, "26"),
    kind: "handoff",
    summary: "The archive has a working foundation.",
    completed: [
      "Built the local record index and project browser.",
      "Added literal note search with cursor pagination.",
      "Connected a read-only store; existing records remain untouched.",
    ],
    unfinished: [
      "Keyboard review of the record inspector.",
      "Exercise the export flow with a large handoff.",
    ],
    next_actions: [
      "Review the search experience with real field notes.",
      "Verify context briefs in a fresh conversation.",
    ],
    blockers: [],
    verification: [
      "Recorded on 26 Nov: 28 unit tests passed.",
      "Recorded on 26 Nov: frontend production build passed.",
    ],
    sources: ["src/archive.ts", "docs/verification-2026-11-26.md"],
  },
  {
    ...common(2, "25"),
    kind: "note",
    title: "Search should feel like looking through a notebook",
    content:
      "Researchers remember fragments, not filenames. Match the words they actually type, including punctuation.\n\nPreserve literal punctuation such as [field] and (draft). Search must never interpret regular expressions. Show enough context to help a person recognize the right note, then open the complete original.\n\nA quiet, chronological list feels more useful than a wall of search scores.",
    author_type: "user",
    sources: ["research/interviews/03.md", "docs/search-principles.md"],
  },
  {
    ...common(3, "24"),
    kind: "decision",
    decision: "Keep the archive offline-first",
    rationale:
      "Fieldwork often happens beyond reliable connectivity. A local archive keeps research available, removes a hosted dependency, and lets the user decide what context leaves their computer.",
    supersedes: null,
    sources: ["docs/architecture.md", "research/field-constraints.md"],
  },
  {
    ...common(4, "23"),
    kind: "decision",
    decision: "Use literal search for the first release",
    rationale:
      "Predictable substring matching is easier to trust and verify than a ranking model. We can revisit richer search after observing real retrieval patterns.",
    supersedes: rid(7),
    sources: ["docs/search-principles.md"],
  },
  {
    ...common(5, "22"),
    kind: "note",
    title: "A small brief is more useful than a full transcript",
    content:
      "A handoff should carry the intent, settled decisions, and the next practical step. Let people choose the records they want to take forward.\n\nAlways preserve the source and the date. A test result from last week is a record of that moment, not a promise about today.",
    sources: ["research/context-sessions.md"],
  },
  {
    ...common(6, "21"),
    kind: "decision",
    decision: "Keep context selection deliberate",
    rationale:
      "Make a brief from selected full records. Do not auto-include every memory or pass truncated previews to another session. A person should be able to see and remove each item before export.",
    supersedes: null,
    sources: ["docs/context-brief.md"],
  },
  {
    ...common(7, "19"),
    kind: "decision",
    decision: "Start with fuzzy note matching",
    rationale:
      "The initial prototype used approximate matching to find related phrases. Early trials surfaced surprising matches and made exact phrases difficult to retrieve.",
    supersedes: null,
    superseded_by: rid(4),
    sources: ["experiments/search-v1.md"],
  },
  {
    ...common(8, "18"),
    kind: "note",
    title: "What belongs in a project memory?",
    content:
      "Record durable facts, the reason behind a decision, and a useful handoff. Keep ephemeral chatter out of the archive. Sources should point to evidence someone can check later.\n\nNames can change. Project identity should stay attached to its ID.",
    sources: ["docs/project-memory.md"],
  },
  {
    ...common(9, "17"),
    kind: "handoff",
    summary: "Research model and first prototype are ready.",
    completed: [
      "Defined notes, decisions, and handoffs.",
      "Created a synthetic fieldwork project.",
    ],
    unfinished: [
      "Read-only native integration.",
      "Literal search and pagination.",
    ],
    next_actions: ["Build a usable project browser."],
    blockers: ["Waiting for store schema review."],
    verification: ["Recorded on 17 Nov: schema tests passed."],
    sources: ["docs/prototype.md"],
  },
  {
    ...common(10, "25", 1),
    kind: "note",
    title: "A shared vocabulary for the community garden",
    content:
      "The volunteer team uses beds, plots, and patches interchangeably. Keep the interface language close to what people say: garden beds for allocation, harvest log for outcomes.",
    author_type: "user",
    sources: ["research/garden-workshop.md"],
  },
  {
    ...common(11, "23", 1),
    kind: "decision",
    decision: "Use a weekly harvest log",
    rationale:
      "Weekly entries fit the volunteer rhythm and avoid asking each person to maintain a daily diary.",
    supersedes: null,
    sources: ["docs/garden-decisions.md"],
  },
  {
    ...common(12, "20", 1),
    kind: "handoff",
    summary: "The first volunteer workshop is documented.",
    completed: ["Mapped the garden beds.", "Interviewed six volunteers."],
    unfinished: ["Validate the harvest form."],
    next_actions: ["Run a paper prototype session on Saturday."],
    blockers: [],
    verification: [
      "Recorded on 20 Nov: workshop notes checked against recordings.",
    ],
    sources: ["research/garden-workshop.md"],
  },
];
// Additional synthetic review notes preserve ordinary notebook ordering.
for (const [n, head, previous] of [
  [13, "b", null],
  [14, "a", rid(13)],
] as const) {
  records.push({
    ...common(n, "26"),
    kind: "note",
    title: `Code review: ${n === 14 ? "Pagination follow-up" : "Archive pagination"}`,
    content:
      n === 14
        ? "The follow-up preserves scoped cursors. Keyboard verification remains pending."
        : "The original pagination review found a cursor scope issue.",
    sources: ["src/archive.ts", "tests/pagination.test.ts"],
    review: {
      format: "project-memory-review-v1",
      repository: "fieldwork-atlas",
      base_commit: "c".repeat(40),
      head_commit: head.repeat(40),
      summary: n === 14 ? "Pagination follow-up" : "Archive pagination",
      previous_review_id: previous,
      checks: [
        {
          name: "Pagination regression suite",
          outcome: "passed",
          result: "Recorded on 26 Nov: scoped cursor tests passed.",
        },
        {
          name: "Keyboard walkthrough",
          outcome: "not_run",
          result: "A manual keyboard walkthrough is still needed.",
        },
      ],
      findings: [
        {
          id: "cursor-scope",
          severity: "medium",
          title: "Keep cursors attached to their project",
          path: "src/archive.ts",
          line: 42,
          evidence: "The original cursor omitted the project identifier.",
          correction:
            "Include project identity in cursor validation and verify keyboard pagination.",
          status: n === 14 ? "needs_verification" : "open",
        },
      ],
    },
  });
}
const projectSeeds = [
  {
    name: "Fieldwork Atlas",
    purpose:
      "A considered home for research notes, settled decisions, and the work still ahead.",
    directory: "~/Projects/fieldwork-atlas",
  },
  {
    name: "Common Ground",
    purpose: "Tools and shared knowledge for a neighborhood community garden.",
    directory: "~/Projects/common-ground",
  },
  {
    name: "Quiet Hours",
    purpose: "An exploration of calmer ways to plan a working day.",
    directory: null,
  },
];
const projects: Project[] = projectSeeds.map((seed, i) => {
  const list = records.filter((record) => record.project_id === projectIds[i]);
  return {
    ...seed,
    id: projectIds[i],
    created_at: "2026-11-01T08:00:00Z",
    record_count: list.length,
    note_count: list.filter((r) => r.kind === "note").length,
    decision_count: list.filter((r) => r.kind === "decision").length,
    active_decision_count: list.filter(
      (r) => r.kind === "decision" && !r.superseded_by,
    ).length,
    handoff_count: list.filter((r) => r.kind === "handoff").length,
    last_activity: list[0]?.created_at ?? null,
  };
});
function preview(record: MemoryRecord): Preview {
  const text =
    record.kind === "note"
      ? record.content
      : record.kind === "decision"
        ? record.decision
        : record.summary;
  return {
    id: record.id,
    project_id: record.project_id,
    kind: record.kind,
    created_at: record.created_at,
    author_type: record.author_type,
    title: record.kind === "note" ? record.title : null,
    preview: text.slice(0, 300),
    content_truncated: text.length > 300,
    full_record_required: true,
    supersedes: record.kind === "decision" ? record.supersedes : null,
    superseded_by: record.superseded_by,
    ...(record.kind === "handoff"
      ? { next_actions: record.next_actions, next_actions_truncated: false }
      : {}),
  };
}
function page<T>(
  items: T[],
  args: Record<string, unknown>,
  key: string,
): Page<T> {
  const limit = Math.min(50, Math.max(1, Number(args.limit) || 20));
  let offset = 0;
  if (args.cursor) {
    const parsed = JSON.parse(decodeURIComponent(String(args.cursor))) as {
      key: string;
      offset: number;
    };
    if (parsed.key !== key)
      throw new Error("This page belongs to another view.");
    offset = parsed.offset;
  }
  const next =
    offset + limit < items.length
      ? encodeURIComponent(JSON.stringify({ key, offset: offset + limit }))
      : null;
  return {
    items: structuredClone(items.slice(offset, offset + limit)),
    next_cursor: next,
    truncated: next !== null,
  };
}
export function createSampleAdapter(): MemoryAdapter {
  const session = "synthetic-workspace";
  const connection: ConnectionInfo = {
    status: "connected",
    path: null,
    default_path: null,
    session_id: session,
    error: null,
  };
  return {
    sample: true,
    bootstrap: async () => connection,
    chooseDatabase: async () => null,
    setupPreview: async (args) => sampleSetup(args),
    async query<T>(
      sessionId: string,
      operation: string,
      args: Record<string, unknown>,
    ): Promise<T> {
      if (sessionId !== session)
        throw { code: "STALE_SESSION", message: "Sample session expired." };
      if (operation === "studio_list_projects" || operation === "list_projects")
        return page(projects, args, operation) as T;
      const project = projects.find((p) => p.id === args.project_id);
      if (!project) throw new Error("Project not found.");
      const scoped = records.filter((r) => r.project_id === project.id);
      if (operation === "get_review_history")
        return sampleReviews(project, scoped, args) as T;
      if (operation === "get_memory_health")
        return sampleHealth(project, scoped, args) as T;
      if (operation === "get_record") {
        const record = scoped.find((r) => r.id === args.record_id);
        if (!record) throw new Error("Record not found.");
        return structuredClone(record) as T;
      }
      const key = JSON.stringify([
        operation,
        project.id,
        args.kind ?? null,
        args.include_superseded ??
          (operation === "search_memory" ? false : true),
        args.query ?? "",
      ]);
      if (operation === "studio_list_records")
        return page(
          scoped
            .filter(
              (r) =>
                (!args.kind || r.kind === args.kind) &&
                (args.include_superseded !== false || !r.superseded_by),
            )
            .map(preview),
          args,
          key,
        ) as T;
      if (operation === "search_notes") {
        const query = String(args.query ?? "").toLowerCase();
        return page(
          scoped
            .filter(
              (r) =>
                r.kind === "note" &&
                (r.title.toLowerCase().includes(query) ||
                  r.content.toLowerCase().includes(query)),
            )
            .map(preview),
          args,
          key,
        ) as T;
      }
      if (operation === "search_memory") {
        if (!validLiteralQuery(args.query)) throw { code: "VALIDATION_ERROR" };
        return page(
          scoped
            .filter(
              (record) =>
                (!args.kind || record.kind === args.kind) &&
                (args.include_superseded === true || !record.superseded_by) &&
                matchesHumanText(record, String(args.query)),
            )
            .map(preview),
          args,
          key,
        ) as T;
      }
      if (operation === "get_context_bundle")
        return sampleContextBundle(project, scoped, args) as T;
      if (operation === "get_project_context") {
        const context: ProjectContext = {
          project: structuredClone(project),
          active_decisions: {
            items: scoped
              .filter((r) => r.kind === "decision" && !r.superseded_by)
              .map(preview),
            truncated: false,
          },
          latest_handoff: scoped.find((r) => r.kind === "handoff")
            ? preview(scoped.find((r) => r.kind === "handoff")!)
            : null,
          history: page(
            scoped
              .filter(
                (r) => args.include_superseded !== false || !r.superseded_by,
              )
              .map(preview),
            args,
            key,
          ),
          notice: "Synthetic sample records. No live database is connected.",
        };
        return context as T;
      }
      if (operation === "export_project") {
        const result = page(scoped, args, key);
        return {
          project_id: project.id,
          markdown: formatContext(project, result.items),
          record_ids: result.items.map((record) => record.id),
          next_cursor: result.next_cursor,
          truncated: result.truncated,
        } as T;
      }
      throw new Error("Operation is not available in the sample workspace.");
    },
    copy: async (markdown) => {
      if (!navigator.clipboard?.writeText)
        throw new Error("Clipboard unavailable.");
      await navigator.clipboard.writeText(markdown);
    },
    save: async (markdown) => {
      const blob = new Blob([markdown], {
        type: "text/markdown;charset=utf-8",
      });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = "project-context.md";
      document.body.append(anchor);
      anchor.click();
      anchor.remove();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
      return { saved: true, file_name: "project-context.md" };
    },
  };
}
