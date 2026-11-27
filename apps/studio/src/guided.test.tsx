import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import App from "./App";
import { createSampleAdapter } from "./sample";
import { createNativeAdapter } from "./adapter";
import { formatContext } from "./context";
import { sampleReviews } from "./sampleGuided";
import type { MemoryRecord, Page, Project } from "./types";
async function fixture() {
  const adapter = createSampleAdapter();
  const session = (await adapter.bootstrap()).session_id!;
  const project = (
    await adapter.query<Page<Project>>(session, "studio_list_projects", {})
  ).items[0];
  return { adapter, session, project };
}
it("navigates to Reviews and Health with keyboard while preserving a manual brief", async () => {
  const { adapter } = await fixture();
  const u = userEvent.setup();
  render(<App adapter={adapter} />);
  await screen.findByRole("heading", { name: "Fieldwork Atlas" });
  await u.click(
    await screen.findByRole("button", {
      name: /Open record: Keep the archive offline-first/,
    }),
  );
  await u.click(
    await screen.findByRole("button", { name: "Add to context brief" }),
  );
  await u.click(screen.getByRole("button", { name: "Close record" }));
  screen.getByRole("tab", { name: "Search" }).focus();
  await u.keyboard("{ArrowRight}");
  expect(document.activeElement).toBe(
    screen.getByRole("tab", { name: "Reviews" }),
  );
  await screen.findByRole("heading", { name: "Reviews at a recorded moment." });
  await u.keyboard("{End}");
  expect(document.activeElement).toBe(
    screen.getByRole("tab", { name: "Health" }),
  );
  await u.click(screen.getByRole("button", { name: "Inspect health" }));
  await screen.findByText(/Records examined on this page:/);
  expect(
    within(
      screen.getByRole("complementary", { name: "Context brief" }),
    ).getAllByRole("button", { name: /Remove .* from context/ }),
  ).toHaveLength(1);
});
it("paginates complete reviews with explicit head and invalidates results on edit", async () => {
  const { adapter } = await fixture();
  const original = adapter.query;
  const query = vi
    .spyOn(adapter, "query")
    .mockImplementation((s, op, args) =>
      original(
        s,
        op,
        op === "get_review_history" ? { ...args, limit: 1 } : args,
      ),
    );
  const u = userEvent.setup();
  render(<App adapter={adapter} />);
  await screen.findByRole("heading", { name: "Fieldwork Atlas" });
  await u.click(screen.getByRole("tab", { name: "Reviews" }));
  await u.type(
    screen.getByRole("textbox", { name: "Current head for reviews" }),
    "a".repeat(40),
  );
  await u.click(screen.getByRole("button", { name: "Load reviews" }));
  await screen.findByText("Current supplied head");
  expect(query).toHaveBeenCalledWith(
    expect.any(String),
    "get_review_history",
    expect.objectContaining({
      current_head: "a".repeat(40),
      limit: 20,
      max_bytes: 16000,
    }),
  );
  await u.click(screen.getByRole("button", { name: "Load more reviews" }));
  await screen.findByText("Historical head");
  expect(
    screen.getAllByRole("button", { name: /Open record: Code review:/ }),
  ).toHaveLength(2);
  await u.type(
    screen.getByRole("textbox", { name: "Current head for reviews" }),
    "b",
  );
  expect(screen.queryByText("Current supplied head")).toBeNull();
  expect(
    screen.queryByRole("button", { name: "Load more reviews" }),
  ).toBeNull();
});
it("preserves review metadata and only adds the inspected full note after explicit Add", async () => {
  const { adapter, session, project } = await fixture();
  const result = await adapter.query<Page<MemoryRecord>>(
    session,
    "get_review_history",
    { project_id: project.id },
  );
  const markdown = formatContext(project, [result.items[0]]);
  expect(markdown).toContain('"format": "project-memory-review-v1"');
  expect(markdown).toContain('"previous_review_id":');
  const u = userEvent.setup();
  render(<App adapter={adapter} />);
  await screen.findByRole("heading", { name: "Fieldwork Atlas" });
  await u.click(screen.getByRole("tab", { name: "Reviews" }));
  await u.click(screen.getByRole("button", { name: "Load reviews" }));
  await u.click(
    (
      await screen.findAllByRole("button", {
        name: /Open record: Code review:/,
      })
    )[0],
  );
  await screen.findByRole("heading", { name: "Review evidence · historical" });
  expect(screen.getByText("src/archive.ts:42")).toBeTruthy();
  const brief = screen.getByRole("complementary", { name: "Context brief" });
  expect(
    within(brief).queryByRole("button", { name: /Remove .* from context/ }),
  ).toBeNull();
  await u.click(screen.getByRole("button", { name: "Add to context brief" }));
  expect(
    within(brief).getAllByRole("button", { name: /Remove .* from context/ }),
  ).toHaveLength(1);
});
it("discards pending health after head edit and validates the next request", async () => {
  const { adapter } = await fixture();
  const original = adapter.query;
  let release: (() => void) | undefined;
  adapter.query = async (s, op, args) => {
    if (op === "get_memory_health")
      await new Promise<void>((r) => {
        release = r;
      });
    return original(s, op, args);
  };
  const u = userEvent.setup();
  render(<App adapter={adapter} />);
  await screen.findByRole("heading", { name: "Fieldwork Atlas" });
  await u.click(screen.getByRole("tab", { name: "Health" }));
  await u.click(screen.getByRole("button", { name: "Inspect health" }));
  await waitFor(() => expect(release).toBeTypeOf("function"));
  await u.type(
    screen.getByRole("textbox", { name: "Current head for health" }),
    "b",
  );
  await act(async () => release?.());
  expect(screen.queryByText(/Records examined on this page:/)).toBeNull();
  await u.click(screen.getByRole("button", { name: "Inspect health" }));
  expect((await screen.findByRole("alert")).textContent).toContain("40 or 64");
});
it("previews and copies synthetic setup JSON then invalidates configuration on edit", async () => {
  const { adapter } = await fixture();
  const copy = vi.spyOn(adapter, "copy").mockResolvedValue();
  const u = userEvent.setup();
  render(<App adapter={adapter} />);
  await screen.findByRole("heading", { name: "Fieldwork Atlas" });
  await u.click(screen.getByRole("button", { name: "Guided setup" }));
  await u.type(
    screen.getByRole("textbox", { name: "Database path" }),
    "/synthetic/memory.sqlite",
  );
  await u.type(
    screen.getByRole("textbox", { name: "Executable path" }),
    "/synthetic/project-memory-mcp",
  );
  await u.click(screen.getByRole("button", { name: "Preview setup" }));
  expect(
    await screen.findByText(/No real filesystem was checked/),
  ).toBeTruthy();
  await u.click(screen.getByRole("button", { name: "Copy configuration" }));
  expect(
    JSON.parse(copy.mock.calls[0][0]).mcpServers["project-memory"].command,
  ).toBe("/synthetic/project-memory-mcp");
  await u.click(
    screen.getByRole("checkbox", { name: "Read-only client configuration" }),
  );
  expect(
    screen.queryByRole("button", { name: "Copy configuration" }),
  ).toBeNull();
});
it("uses exact snake_case native setup arguments", async () => {
  const calls: unknown[] = [];
  const adapter = createNativeAdapter(async (command, args) => {
    calls.push([command, args]);
    return {};
  });
  await adapter.setupPreview({
    database: "/tmp/archive.sqlite",
    executable: "/tmp/server",
    policy_path: "/tmp/policy.json",
    read_only: true,
  });
  expect(calls).toEqual([
    [
      "preview_setup",
      {
        database: "/tmp/archive.sqlite",
        executable: "/tmp/server",
        policy_path: "/tmp/policy.json",
        read_only: true,
      },
    ],
  ]);
});

it.each(["reviews", "health"] as const)(
  "discards late %s after project navigation",
  async (mode) => {
    const { adapter } = await fixture();
    const original = adapter.query;
    let release: (() => void) | undefined;
    adapter.query = async (s, op, args) => {
      if (
        op === (mode === "reviews" ? "get_review_history" : "get_memory_health")
      )
        await new Promise<void>((r) => {
          release = r;
        });
      return original(s, op, args);
    };
    const u = userEvent.setup();
    render(<App adapter={adapter} />);
    await screen.findByRole("heading", { name: "Fieldwork Atlas" });
    await u.click(
      screen.getByRole("tab", {
        name: mode === "reviews" ? "Reviews" : "Health",
      }),
    );
    await u.click(
      screen.getByRole("button", {
        name: mode === "reviews" ? "Load reviews" : "Inspect health",
      }),
    );
    await waitFor(() => expect(release).toBeTypeOf("function"));
    await u.click(
      screen.getByRole("button", { name: "Open project: Common Ground" }),
    );
    await act(async () => release?.());
    await screen.findByRole("heading", { name: "Common Ground" });
    expect(screen.queryByText(/Records examined on this page:/)).toBeNull();
    expect(
      screen.queryByRole("button", { name: /Open record: Code review:/ }),
    ).toBeNull();
  },
);

it("discards pending setup after path edit and after leaving its workspace", async () => {
  const { adapter } = await fixture();
  const original = adapter.setupPreview;
  let release: (() => void) | undefined;
  adapter.setupPreview = async (args) => {
    await new Promise<void>((r) => {
      release = r;
    });
    return original(args);
  };
  const u = userEvent.setup();
  render(<App adapter={adapter} />);
  await screen.findByRole("heading", { name: "Fieldwork Atlas" });
  await u.click(screen.getByRole("button", { name: "Guided setup" }));
  await u.type(
    screen.getByRole("textbox", { name: "Database path" }),
    "/synthetic/memory.sqlite",
  );
  await u.type(
    screen.getByRole("textbox", { name: "Executable path" }),
    "/synthetic/server",
  );
  await u.click(screen.getByRole("button", { name: "Preview setup" }));
  await waitFor(() => expect(release).toBeTypeOf("function"));
  await u.type(
    screen.getByRole("textbox", { name: "Policy path" }),
    "/synthetic/policy.json",
  );
  await act(async () => release?.());
  expect(
    screen.queryByRole("button", { name: "Copy configuration" }),
  ).toBeNull();
  await u.click(screen.getByRole("button", { name: "Preview setup" }));
  await u.click(
    screen.getByRole("button", { name: "Open project: Common Ground" }),
  );
  await act(async () => release?.());
  expect(
    screen.queryByRole("button", { name: "Copy configuration" }),
  ).toBeNull();
});

it("offers guided setup before any archive exists", async () => {
  const { adapter } = await fixture();
  adapter.sample = false;
  adapter.bootstrap = async () => ({
    status: "missing",
    path: null,
    default_path: "/synthetic/missing.sqlite",
    session_id: null,
    error: null,
  });
  const u = userEvent.setup();
  render(<App adapter={adapter} />);
  await u.click(await screen.findByRole("button", { name: "Guided setup" }));
  expect(screen.getByRole("textbox", { name: "Database path" })).toBeTruthy();
  expect(
    screen.getByText(/Studio keeps its own local read-only session/),
  ).toBeTruthy();
});

it("sample health pagination includes no-signal records, scopes cursors to the supplied head, and accounts for canonical bytes", async () => {
  const { adapter, session, project } = await fixture();
  type Health = import("./types").MemoryHealth;
  const args = {
    project_id: project.id,
    limit: 1,
    max_bytes: 4096,
    current_head: "a".repeat(40),
  };
  const first = await adapter.query<Health>(session, "get_memory_health", args);
  expect(first.items).toHaveLength(1);
  expect(first.items[0].signals).toHaveLength(0);
  expect(first.records_examined).toBe(1);
  expect(first.next_cursor).toBeTruthy();
  const size = new TextEncoder().encode(
    JSON.stringify({
      resultType: "complete",
      content: [{ type: "text", text: JSON.stringify(first) }],
      structuredContent: first,
      isError: false,
    }),
  ).byteLength;
  expect(first.budget.used_bytes).toBe(size);
  expect(size).toBeLessThanOrEqual(4096);
  const second = await adapter.query<Health>(session, "get_memory_health", {
    ...args,
    cursor: first.next_cursor,
  });
  expect(second.items[0].record_id).not.toBe(first.items[0].record_id);
  expect(second.summary).toEqual(first.summary);
  await expect(
    adapter.query(session, "get_memory_health", {
      ...args,
      current_head: "b".repeat(40),
      cursor: first.next_cursor,
    }),
  ).rejects.toMatchObject({ code: "INVALID_CURSOR" });
  for (const invalid of [
    { current_head: "A".repeat(40) },
    { limit: 51 },
    { max_bytes: 4095 },
  ])
    await expect(
      adapter.query(session, "get_memory_health", { ...args, ...invalid }),
    ).rejects.toMatchObject({ code: "VALIDATION_ERROR" });
});

it("sample review validation rejects mismatched head cursors and incomplete budgets without returning partial notes", async () => {
  const { adapter, session, project } = await fixture();
  const args = {
    project_id: project.id,
    current_head: "a".repeat(40),
    limit: 1,
  };
  const first = await adapter.query<import("./types").ReviewPage>(
    session,
    "get_review_history",
    args,
  );
  await expect(
    adapter.query(session, "get_review_history", {
      ...args,
      cursor: first.next_cursor,
      current_head: "b".repeat(40),
    }),
  ).rejects.toMatchObject({ code: "INVALID_CURSOR" });
  expect(() =>
    sampleReviews(
      project,
      [{ ...first.items[0], content: "full record ".repeat(1000) }],
      { ...args, max_bytes: 4096 },
    ),
  ).toThrow();
  await expect(
    adapter.query(session, "get_review_history", { ...args, limit: 21 }),
  ).rejects.toMatchObject({ code: "VALIDATION_ERROR" });
});
it.each(["get_memory_health", "get_review_history"])(
  "rejects unknown sample %s arguments",
  async (operation) => {
    const { adapter, session, project } = await fixture();
    await expect(
      adapter.query(session, operation, {
        project_id: project.id,
        unexpected: true,
      }),
    ).rejects.toMatchObject({ code: "VALIDATION_ERROR" });
  },
);
it("treats null optional review head/cursor like omitted values", async () => {
  const { adapter, session, project } = await fixture();
  const page = await adapter.query<import("./types").ReviewPage>(
    session,
    "get_review_history",
    { project_id: project.id, current_head: null, cursor: null },
  );
  expect(page.items[0].freshness).toBe("unknown");
});
it("rejects commit IDs with trailing newline in UI and sample validation", async () => {
  const { validHead } = await import("./EvidenceViews");
  expect(validHead("a".repeat(40) + "\n")).toBe(false);
  const { adapter, session, project } = await fixture();
  await expect(
    adapter.query(session, "get_review_history", {
      project_id: project.id,
      current_head: "a".repeat(40) + "\n",
    }),
  ).rejects.toMatchObject({ code: "VALIDATION_ERROR" });
});
it.each(["reviews", "health"] as const)(
  "drops late %s replies when the session changes",
  async (mode) => {
    const { default: EvidenceViews } = await import("./EvidenceViews");
    const { adapter, session, project } = await fixture();
    const original = adapter.query;
    let release: (() => void) | undefined;
    adapter.query = async (s, op, args) => {
      await new Promise<void>((r) => {
        release = r;
      });
      return original(s, op, args);
    };
    const stale = vi.fn(() => false);
    const props = {
      adapter,
      session,
      project,
      mode,
      onOpen: () => {},
      onStale: stale,
    };
    const { rerender } = render(<EvidenceViews {...props} />);
    const u = userEvent.setup();
    await u.click(
      screen.getByRole("button", {
        name: mode === "reviews" ? "Load reviews" : "Inspect health",
      }),
    );
    await waitFor(() => expect(release).toBeTypeOf("function"));
    rerender(<EvidenceViews {...props} session="replacement" />);
    await act(async () => release?.());
    expect(screen.queryByText(/Page budget:/)).toBeNull();
    expect(stale).not.toHaveBeenCalled();
  },
);
it("shows safe cursor failure and keeps a fresh-first-page recovery action", async () => {
  const { adapter } = await fixture();
  const original = adapter.query;
  adapter.query = (s, op, args) => {
    if (op === "get_memory_health")
      return Promise.reject({
        code: "INVALID_CURSOR",
        message: "raw private server detail",
      });
    return original(s, op, args);
  };
  const u = userEvent.setup();
  render(<App adapter={adapter} />);
  await screen.findByRole("heading", { name: "Fieldwork Atlas" });
  await u.click(screen.getByRole("tab", { name: "Health" }));
  await u.click(screen.getByRole("button", { name: "Inspect health" }));
  expect((await screen.findByRole("alert")).textContent).toContain(
    "fresh first page",
  );
  expect(screen.queryByText("raw private server detail")).toBeNull();
  expect(screen.getByRole("button", { name: "Start again" })).toBeTruthy();
});

it("keeps review history revision and supplied-head evidence through inspector, Add, and Markdown", async () => {
  const { adapter } = await fixture();
  const copy = vi.spyOn(adapter, "copy").mockResolvedValue();
  const u = userEvent.setup();
  render(<App adapter={adapter} />);
  await screen.findByRole("heading", { name: "Fieldwork Atlas" });
  await u.click(screen.getByRole("tab", { name: "Reviews" }));
  await u.type(
    screen.getByRole("textbox", { name: "Current head for reviews" }),
    "a".repeat(40),
  );
  await u.click(screen.getByRole("button", { name: "Load reviews" }));
  await u.click(
    await screen.findByRole("button", {
      name: "Open record: Code review: Archive pagination",
    }),
  );
  const inspector = screen.getByRole("dialog", {
    name: "Complete memory record",
  });
  expect(
    await within(inspector).findByText("f6181db9-c091-49a5-a658-000000000014"),
  ).toBeTruthy();
  expect(
    within(inspector).getByText("Historical head · loaded snapshot comparison"),
  ).toBeTruthy();
  expect(
    within(inspector).getByText(/Supplied head for this snapshot:/).textContent,
  ).toContain("a".repeat(40));
  expect(
    within(inspector).queryByText("No revision in this result"),
  ).toBeNull();
  await u.click(
    within(inspector).getByRole("button", { name: "Add to context brief" }),
  );
  await u.click(
    within(inspector).getByRole("button", { name: "Close record" }),
  );
  // Editing today's input cannot reinterpret the selected historical comparison.
  await u.clear(
    screen.getByRole("textbox", { name: "Current head for reviews" }),
  );
  await u.type(
    screen.getByRole("textbox", { name: "Current head for reviews" }),
    "b".repeat(40),
  );
  await u.click(screen.getByRole("button", { name: "Copy Markdown" }));
  const markdown = copy.mock.calls[0][0];
  expect(markdown).toContain(
    '"revised_by": "f6181db9-c091-49a5-a658-000000000014"',
  );
  expect(markdown).toContain('"freshness": "stale"');
  expect(markdown).toContain('"current_head": "' + "a".repeat(40) + '"');
  expect(markdown).toContain("loaded history snapshot");
  expect(markdown).toContain(
    "The original pagination review found a cursor scope issue.",
  );
  expect(markdown).toContain("tests/pagination.test.ts");
});

it("keeps an opened complete review snapshot when an older full-record request finishes late", async () => {
  const { adapter } = await fixture();
  const original = adapter.query;
  let release: (() => void) | undefined;
  adapter.query = async (session, operation, args) => {
    if (
      operation === "get_record" &&
      String(args.record_id).endsWith("000000000003")
    )
      await new Promise<void>((resolve) => {
        release = resolve;
      });
    return original(session, operation, args);
  };
  const u = userEvent.setup();
  render(<App adapter={adapter} />);
  await screen.findByRole("heading", { name: "Fieldwork Atlas" });
  await u.click(
    await screen.findByRole("button", {
      name: "Open record: Keep the archive offline-first",
    }),
  );
  await waitFor(() => expect(release).toBeTypeOf("function"));
  await u.click(screen.getByRole("button", { name: "Close record" }));
  await u.click(screen.getByRole("tab", { name: "Reviews" }));
  await u.click(screen.getByRole("button", { name: "Load reviews" }));
  await u.click(
    await screen.findByRole("button", {
      name: "Open record: Code review: Archive pagination",
    }),
  );
  await act(async () => release?.());
  const inspector = screen.getByRole("dialog", {
    name: "Complete memory record",
  });
  expect(
    within(inspector).getByRole("heading", {
      name: "Code review: Archive pagination",
    }),
  ).toBeTruthy();
  expect(
    within(inspector).getByText("f6181db9-c091-49a5-a658-000000000014"),
  ).toBeTruthy();
  expect(
    within(inspector).getByText(
      "Head not supplied · loaded snapshot comparison",
    ),
  ).toBeTruthy();
  expect(
    within(inspector).queryByText(
      "Matching supplied head · loaded snapshot comparison",
    ),
  ).toBeNull();
});

it.each(["Escape", "Enter"] as const)(
  "restores record opener focus after keyboard %s closes the inspector",
  async (key) => {
    const { adapter } = await fixture();
    const u = userEvent.setup();
    render(<App adapter={adapter} />);
    await screen.findByRole("heading", { name: "Fieldwork Atlas" });
    const opener = await screen.findByRole("button", {
      name: "Open record: Keep the archive offline-first",
    });
    opener.focus();
    await u.keyboard("{Enter}");
    const close = await screen.findByRole("button", { name: "Close record" });
    close.focus();
    await u.keyboard(`{${key}}`);
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(document.activeElement).toBe(opener);
  },
);
