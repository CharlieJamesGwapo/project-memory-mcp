import type {
  MemoryHealth,
  MemoryRecord,
  Note,
  Project,
  ReviewPage,
  SetupInput,
  SetupReport,
} from "./types";
const fail = (code: string): never => {
  throw { code };
};
const validHead = (head: unknown) =>
  head == null ||
  (typeof head === "string" &&
    [40, 64].includes(head.length) &&
    /^[0-9a-f]+$/.test(head));
function measured<T extends { budget: { used_bytes: number } }>(value: T): T {
  for (;;) {
    const size = new TextEncoder().encode(
      JSON.stringify({
        resultType: "complete",
        content: [{ type: "text", text: JSON.stringify(value) }],
        structuredContent: value,
        isError: false,
      }),
    ).byteLength;
    if (size === value.budget.used_bytes) return value;
    value.budget.used_bytes = size;
  }
}
function bounded<
  T,
  R extends {
    items: T[];
    next_cursor: string | null;
    truncated: boolean;
    budget: { max_bytes: number; used_bytes: number };
  },
>(
  all: T[],
  args: Record<string, unknown>,
  scope: string,
  result: R,
  maxLimit: number,
): R {
  if (
    Object.keys(args).some(
      (key) =>
        ![
          "project_id",
          "current_head",
          "cursor",
          "limit",
          "max_bytes",
        ].includes(key),
    )
  )
    fail("VALIDATION_ERROR");
  const limit =
    args.limit === undefined ? (maxLimit === 20 ? 10 : 20) : args.limit;
  const maxBytes = args.max_bytes === undefined ? 16000 : args.max_bytes;
  if (
    !Number.isInteger(limit) ||
    Number(limit) < 1 ||
    Number(limit) > maxLimit ||
    !Number.isInteger(maxBytes) ||
    Number(maxBytes) < 4096 ||
    Number(maxBytes) > 65536 ||
    !validHead(args.current_head)
  )
    fail("VALIDATION_ERROR");
  let offset = 0;
  if (args.cursor != null) {
    try {
      const parsed = JSON.parse(decodeURIComponent(String(args.cursor)));
      if (
        parsed.scope !== scope ||
        !Number.isInteger(parsed.offset) ||
        parsed.offset < 1 ||
        parsed.offset >= all.length
      )
        fail("INVALID_CURSOR");
      offset = parsed.offset;
    } catch {
      fail("INVALID_CURSOR");
    }
  }
  result.budget.max_bytes = Number(maxBytes);
  const update = () => {
    const next = offset + result.items.length;
    result.next_cursor =
      next < all.length
        ? encodeURIComponent(JSON.stringify({ scope, offset: next }))
        : null;
    result.truncated = result.next_cursor !== null;
    if ("records_examined" in result)
      result.records_examined = result.items.length;
    return measured(result);
  };
  update();
  for (const item of all.slice(offset, offset + Number(limit))) {
    result.items.push(structuredClone(item));
    if (update().budget.used_bytes > Number(maxBytes)) {
      result.items.pop();
      update();
      break;
    }
  }
  if (
    (offset < all.length && !result.items.length) ||
    result.budget.used_bytes > Number(maxBytes)
  )
    fail("BUDGET_EXCEEDED");
  return structuredClone(result);
}
export function sampleReviews(
  project: Project,
  records: MemoryRecord[],
  args: Record<string, unknown>,
): ReviewPage {
  const notes = records
    .filter(
      (record): record is Note => record.kind === "note" && !!record.review,
    )
    .slice()
    .reverse();
  const items = notes.map((note) => ({
    ...note,
    review: {
      ...note.review!,
      revised_by:
        notes.find((other) => other.review!.previous_review_id === note.id)
          ?.id ?? null,
    },
    freshness: (args.current_head == null
      ? "unknown"
      : note.review!.head_commit === args.current_head
        ? "current"
        : "stale") as Note["freshness"],
  }));
  return bounded<Note, ReviewPage>(
    items,
    args,
    JSON.stringify(["reviews", project.id, args.current_head ?? null]),
    {
      items: [],
      next_cursor: null,
      truncated: false,
      budget: { max_bytes: 16000, used_bytes: 0 },
      notice:
        "Synthetic supplied review evidence. Freshness compares supplied commit IDs only; no repository or checks were inspected.",
    },
    20,
  );
}
export function sampleHealth(
  project: Project,
  records: MemoryRecord[],
  args: Record<string, unknown>,
): MemoryHealth {
  const reviews = records.filter(
    (record): record is Note => record.kind === "note" && !!record.review,
  );
  const latest = reviews.filter(
    (note) =>
      !reviews.some((other) => other.review!.previous_review_id === note.id),
  );
  const handoff = records.find((record) => record.kind === "handoff");
  const items = records.map((record) => {
    const signals = [];
    if (!record.sources.length)
      signals.push({
        code: "missing_sources",
        message: "No source references were saved with this record.",
      });
    if (
      record.kind === "note" &&
      record.review &&
      latest.some((note) => note.id === record.id)
    ) {
      if (
        record.review.findings.some(
          (f) => f.status === "open" || f.status === "needs_verification",
        )
      )
        signals.push({
          code: "review_follow_up",
          message:
            "The latest review has findings marked open or needs verification.",
        });
      if (args.current_head && args.current_head !== record.review.head_commit)
        signals.push({
          code: "review_head_mismatch",
          message: "The saved head differs from the supplied current head.",
        });
    }
    return { record_id: record.id, kind: record.kind, signals };
  });
  const active = records.filter(
    (r) => r.kind === "decision" && !r.superseded_by,
  ).length;
  return bounded<MemoryHealth["items"][number], MemoryHealth>(
    items,
    args,
    JSON.stringify(["health", project.id, args.current_head ?? null]),
    {
      project_id: project.id,
      summary: {
        record_count: records.length,
        active_decision_count: active,
        review_count: reviews.length,
        current_review_count: latest.length,
        open_finding_count: latest
          .flatMap((r) => r.review!.findings)
          .filter(
            (f) => f.status === "open" || f.status === "needs_verification",
          ).length,
        missing_source_count: records.filter((r) => !r.sources.length).length,
        latest_handoff_id: handoff?.id ?? null,
        suggestions:
          active > 1
            ? [
                {
                  code: "review_active_decisions",
                  message:
                    "Several active decisions are saved. Review their scope when useful; this is not evidence of a semantic conflict.",
                },
              ]
            : [],
      },
      items: [],
      records_examined: 0,
      next_cursor: null,
      truncated: false,
      budget: { max_bytes: 16000, used_bytes: 0 },
      notice:
        "Synthetic metadata inspection. Source references were not opened, historical checks were not rerun, and no semantic or safety assessment was performed. Follow all pages for this snapshot.",
    },
    50,
  );
}
export function sampleSetup(args: SetupInput): SetupReport {
  const absolute = (path: string) =>
    path.length <= 4096 &&
    !/[\u0000-\u001f\u007f]/.test(path) &&
    (/^\//.test(path) || /^[A-Za-z]:[\\/]/.test(path));
  const valid =
    absolute(args.database) &&
    absolute(args.executable) &&
    (!args.policy_path || absolute(args.policy_path));
  return {
    status: valid ? "ready" : "action_required",
    checks: [
      {
        code: "synthetic_paths",
        status: valid ? "passed" : "action_required",
        message: valid
          ? "Synthetic absolute paths accepted for this illustration."
          : "Enter explicit absolute database, executable, and optional policy paths.",
      },
    ],
    configuration: valid
      ? {
          mcpServers: {
            "project-memory": {
              command: args.executable,
              args: [
                "--db",
                args.database,
                ...(args.policy_path ? ["--policy", args.policy_path] : []),
                ...(args.read_only ? ["--read-only"] : []),
              ],
            },
          },
        }
      : null,
    next_actions: valid
      ? ["Review the JSON, then add it to your client explicitly."]
      : ["Provide absolute paths and preview again."],
    notice:
      "Synthetic preview. No real filesystem was checked, no executable was run, and no client configuration was written.",
  };
}
