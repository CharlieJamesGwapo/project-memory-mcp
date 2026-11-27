import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { createSampleAdapter } from "./sample";
import SearchMemory from "./SearchMemory";
import ContextSuggestions from "./ContextSuggestions";
import type {
  ContextBundle,
  MemoryRecord,
  Page,
  Preview,
  Project,
} from "./types";
async function fixture() {
  const adapter = createSampleAdapter();
  const connection = await adapter.bootstrap();
  const projects = await adapter.query<Page<Project>>(
    connection.session_id!,
    "studio_list_projects",
    {},
  );
  return {
    adapter,
    session: connection.session_id!,
    project: projects.items[0],
    other: projects.items[1],
  };
}
describe("unified sample retrieval", () => {
  it("searches decoded human text across kinds with literal punctuation and current-record defaults", async () => {
    const { adapter, session, project } = await fixture();
    const args = { project_id: project.id, query: "literal" };
    const page = await adapter.query<Page<Preview>>(
      session,
      "search_memory",
      args,
    );
    expect(new Set(page.items.map((r) => r.kind))).toEqual(
      new Set(["note", "decision", "handoff"]),
    );
    const historical = await adapter.query<Page<Preview>>(
      session,
      "search_memory",
      { project_id: project.id, query: "approximate matching" },
    );
    expect(historical.items).toHaveLength(0);
    const withHistory = await adapter.query<Page<Preview>>(
      session,
      "search_memory",
      {
        project_id: project.id,
        query: "approximate matching",
        include_superseded: true,
      },
    );
    expect(withHistory.items).toHaveLength(1);
    const literal = await adapter.query<Page<Preview>>(
      session,
      "search_memory",
      { project_id: project.id, query: ".*" },
    );
    expect(literal.items).toHaveLength(0);
    const source = await adapter.query<Page<Preview>>(
      session,
      "search_memory",
      { project_id: project.id, query: "docs/architecture.md" },
    );
    expect(source.items).toHaveLength(0);
  });
  it("returns bounded full records and explicit omission metadata", async () => {
    const { adapter, session, project } = await fixture();
    const b = await adapter.query<ContextBundle>(
      session,
      "get_context_bundle",
      { project_id: project.id, max_bytes: 4096, max_records: 8 },
    );
    expect(b.budget.used_bytes).toBeLessThanOrEqual(4096);
    expect(b.budget.estimated_tokens).toBe(Math.ceil(b.budget.used_bytes / 4));
    expect(b.selection.omitted_by_budget).toBeGreaterThan(0);
    expect(b.records.length).toBeGreaterThan(0);
    expect(b.records[0]).toHaveProperty("sources");
    const capped = await adapter.query<ContextBundle>(
      session,
      "get_context_bundle",
      { project_id: project.id, max_records: 1 },
    );
    expect(capped.records).toHaveLength(1);
    expect(capped.records[0].selection_reason).toBe("latest_handoff");
    expect(capped.records[0].kind).toBe("handoff");
    expect(capped.selection.omitted_by_record_limit).toBeGreaterThan(0);
  });
});
describe("retrieval controls", () => {
  it("searches all record kinds, keeps literal query filters with pagination, and opens previews through the full-record inspector callback", async () => {
    const f = await fixture();
    const open = vi.fn();
    const query = vi.spyOn(f.adapter, "query");
    render(<SearchMemory {...f} onOpen={open} onStale={() => false} />);
    const user = userEvent.setup();
    await user.type(
      screen.getByRole("textbox", { name: "Search all memory" }),
      "literal",
    );
    await user.click(screen.getByRole("button", { name: "Search memory" }));
    await screen.findByRole("button", {
      name: /Open record: Use literal search/,
    });
    expect(query).toHaveBeenCalledWith(
      f.session,
      "search_memory",
      expect.objectContaining({
        project_id: f.project.id,
        query: "literal",
        include_superseded: false,
      }),
    );
    await user.click(
      screen.getByRole("button", { name: /Open record: Use literal search/ }),
    );
    expect(open.mock.calls[0][0].kind).toBe("decision");
  });
  it("reviews suggestions without applying them and applies full records only after an explicit action", async () => {
    const f = await fixture();
    const apply = vi.fn(
      (_records: MemoryRecord[], _mode: "add" | "replace") => null,
    );
    render(
      <ContextSuggestions
        {...f}
        disabled={false}
        onApply={apply}
        onStale={() => false}
      />,
    );
    const user = userEvent.setup();
    await user.click(screen.getByText("Suggest context"));
    await user.click(screen.getByRole("button", { name: "Build suggestions" }));
    await screen.findByText(/Retrieval result:/);
    expect(apply).not.toHaveBeenCalled();
    expect(screen.getByText(/Candidate window:/)).toBeTruthy();
    expect(screen.getByText(/Final Markdown/)).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Add suggestions" }));
    expect(apply).toHaveBeenCalledOnce();
    expect(apply.mock.calls[0][0][0]).toHaveProperty("sources");
    expect(apply.mock.calls[0][1]).toBe("add");
    await user.click(screen.getByRole("button", { name: "Replace brief" }));
    expect(apply.mock.calls[1][1]).toBe("replace");
  });
  it("discards late suggestions after project navigation and keeps failures safe", async () => {
    const f = await fixture();
    const original = f.adapter.query;
    let release: (() => void) | undefined;
    f.adapter.query = async (s, op, args) => {
      if (op === "get_context_bundle")
        await new Promise<void>((resolve) => {
          release = resolve;
        });
      return original(s, op, args);
    };
    const apply = vi.fn(
      (_records: MemoryRecord[], _mode: "add" | "replace") => null,
    );
    const { rerender } = render(
      <ContextSuggestions
        {...f}
        disabled={false}
        onApply={apply}
        onStale={() => false}
      />,
    );
    const user = userEvent.setup();
    await user.click(screen.getByText("Suggest context"));
    await user.click(screen.getByRole("button", { name: "Build suggestions" }));
    await waitFor(() => expect(release).toBeTypeOf("function"));
    rerender(
      <ContextSuggestions
        {...f}
        project={f.other}
        disabled={false}
        onApply={apply}
        onStale={() => false}
      />,
    );
    await act(async () => {
      release?.();
    });
    await waitFor(() =>
      expect(
        screen.queryByRole("button", { name: "Add suggestions" }),
      ).toBeNull(),
    );
    expect(apply).not.toHaveBeenCalled();
  });
  it("shows a safe suggestion error and never enables applying a partial result", async () => {
    const f = await fixture();
    f.adapter.query = async () => {
      throw { code: "BUDGET_EXCEEDED", message: "raw server internals" };
    };
    render(
      <ContextSuggestions
        {...f}
        disabled={false}
        onApply={() => null}
        onStale={() => false}
      />,
    );
    const user = userEvent.setup();
    await user.click(screen.getByText("Suggest context"));
    await user.click(screen.getByRole("button", { name: "Build suggestions" }));
    expect(await screen.findByRole("alert")).toHaveProperty(
      "textContent",
      expect.stringContaining("retrieval budget"),
    );
    expect(screen.queryByText("raw server internals")).toBeNull();
    expect(
      screen.queryByRole("button", { name: "Add suggestions" }),
    ).toBeNull();
  });
});

it("uses exact native read operations for unified search and bounded suggestions", async () => {
  const { createNativeAdapter } = await import("./adapter");
  const calls: unknown[] = [];
  const adapter = createNativeAdapter(async (command, args) => {
    calls.push([command, args]);
    return {};
  });
  await adapter.query("session", "search_memory", {
    project_id: "p",
    query: "[field]",
    include_superseded: false,
    limit: 20,
  });
  await adapter.query("session", "get_context_bundle", {
    project_id: "p",
    max_bytes: 16000,
    max_records: 8,
  });
  expect(calls).toEqual([
    [
      "memory_query",
      {
        session_id: "session",
        operation: "search_memory",
        args: {
          project_id: "p",
          query: "[field]",
          include_superseded: false,
          limit: 20,
        },
      },
    ],
    [
      "memory_query",
      {
        session_id: "session",
        operation: "get_context_bundle",
        args: { project_id: "p", max_bytes: 16000, max_records: 8 },
      },
    ],
  ]);
});

it("rejects obsolete search replies after a newer literal query and after a session change", async () => {
  const f = await fixture();
  const original = f.adapter.query;
  let release: (() => void) | undefined;
  f.adapter.query = async (s, op, args) => {
    if (op === "search_memory" && args.query === "notebook")
      await new Promise<void>((resolve) => {
        release = resolve;
      });
    return original(s, op, args);
  };
  const stale = vi.fn(() => false);
  const { rerender } = render(
    <SearchMemory {...f} onOpen={() => {}} onStale={stale} />,
  );
  const user = userEvent.setup();
  const input = screen.getByRole("textbox", { name: "Search all memory" });
  await user.type(input, "notebook");
  await user.click(screen.getByRole("button", { name: "Search memory" }));
  await waitFor(() => expect(release).toBeTypeOf("function"));
  await user.clear(input);
  await user.type(input, "offline-first");
  await user.click(screen.getByRole("button", { name: "Search memory" }));
  await screen.findByRole("button", {
    name: /Open record: Keep the archive offline-first/,
  });
  await act(async () => {
    release?.();
  });
  await waitFor(() =>
    expect(
      screen.queryByRole("button", { name: /Open record: Search should feel/ }),
    ).toBeNull(),
  );
  await user.clear(input);
  await user.type(input, "notebook");
  await user.click(screen.getByRole("button", { name: "Search memory" }));
  rerender(
    <SearchMemory
      {...f}
      session="replacement-session"
      onOpen={() => {}}
      onStale={stale}
    />,
  );
  await act(async () => {
    release?.();
  });
  await waitFor(() =>
    expect(screen.queryByRole("button", { name: /Open record:/ })).toBeNull(),
  );
  expect(stale).not.toHaveBeenCalled();
});

it("uses the context defaults, preserves task literals, and invalidates suggestions when controls change", async () => {
  const f = await fixture();
  const query = vi.spyOn(f.adapter, "query");
  render(
    <ContextSuggestions
      {...f}
      disabled={false}
      onApply={() => null}
      onStale={() => false}
    />,
  );
  const user = userEvent.setup();
  await user.click(screen.getByText("Suggest context"));
  await user.click(screen.getByRole("button", { name: "Build suggestions" }));
  await screen.findByRole("button", { name: "Add suggestions" });
  expect(query).toHaveBeenLastCalledWith(f.session, "get_context_bundle", {
    project_id: f.project.id,
    max_bytes: 16000,
    max_records: 8,
  });
  await user.click(
    screen.getByRole("textbox", { name: "Context task phrase" }),
  );
  await user.paste("[field]");
  expect(screen.queryByRole("button", { name: "Add suggestions" })).toBeNull();
  await user.click(screen.getByRole("button", { name: "Build suggestions" }));
  await screen.findByRole("button", { name: "Add suggestions" });
  expect(query).toHaveBeenLastCalledWith(f.session, "get_context_bundle", {
    project_id: f.project.id,
    query: "[field]",
    max_bytes: 16000,
    max_records: 8,
  });
});

it("uses explicit context presets and advanced custom caps without retrieving or applying on selection", async () => {
  const f = await fixture();
  const query = vi.spyOn(f.adapter, "query");
  const apply = vi.fn(() => null);
  render(<ContextSuggestions {...f} disabled={false} onApply={apply} onStale={() => false} />);
  const user = userEvent.setup();
  await user.click(screen.getByText("Suggest context"));
  expect(screen.getByRole("button", { name: "Balanced" }).getAttribute("aria-pressed")).toBe("true");
  expect((screen.getByText("Advanced limits").closest("details") as HTMLDetailsElement).open).toBe(false);
  query.mockClear();
  for (const [name, bytes, records] of [["Small", 8000, 4], ["Balanced", 16000, 8], ["Expanded", 32000, 12]] as const) {
    await user.click(screen.getByRole("button", { name }));
    expect(query).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Build suggestions" }));
    await screen.findByText(/Retrieval result:/);
    expect(query).toHaveBeenLastCalledWith(f.session, "get_context_bundle", {
      project_id: f.project.id,
      max_bytes: bytes,
      max_records: records,
    });
    expect(apply).not.toHaveBeenCalled();
    query.mockClear();
  }
  await user.click(screen.getByText("Advanced limits"));
  const bytes = screen.getByRole("spinbutton", { name: "Retrieval byte budget" });
  await user.clear(bytes);
  await user.type(bytes, "12000");
  expect(screen.getByText("Custom limits")).toBeTruthy();
  expect(screen.getByRole("button", { name: "Expanded" }).getAttribute("aria-pressed")).toBe("false");
  expect(screen.queryByRole("button", { name: "Add suggestions" })).toBeNull();
  expect(query).not.toHaveBeenCalled();
  await user.click(screen.getByRole("button", { name: "Build suggestions" }));
  await screen.findByText(/Retrieval result:/);
  expect(query).toHaveBeenLastCalledWith(f.session, "get_context_bundle", {
    project_id: f.project.id,
    max_bytes: 12000,
    max_records: 12,
  });
  await user.clear(bytes);
  await user.click(screen.getByText("Advanced limits"));
  query.mockClear();
  await user.click(screen.getByRole("button", { name: "Build suggestions" }));
  expect((await screen.findByRole("alert")).textContent).toContain("4096–65536");
  expect(query).not.toHaveBeenCalled();
});

it("opts into ranked terms explicitly, validates usable terms, and keeps literal defaults", async () => {
  const f = await fixture();
  const query = vi.spyOn(f.adapter, "query");
  render(<ContextSuggestions {...f} disabled={false} onApply={() => null} onStale={() => false} />);
  const user = userEvent.setup();
  await user.click(screen.getByText("Suggest context"));
  expect(screen.queryByRole("radio", { name: "Any task terms" })).toBeNull();
  await user.type(screen.getByRole("textbox", { name: "Context task phrase" }), "offline memory");
  expect((screen.getByRole("radio", { name: "Literal phrase" }) as HTMLInputElement).checked).toBe(true);
  await user.click(screen.getByRole("radio", { name: "Any task terms" }));
  query.mockClear();
  expect(query).not.toHaveBeenCalled();
  await user.click(screen.getByRole("button", { name: "Build suggestions" }));
  await screen.findByText(/Retrieval result:/);
  expect(query).toHaveBeenLastCalledWith(f.session, "get_context_bundle", {
    project_id: f.project.id,
    query: "offline memory",
    query_mode: "terms",
    max_bytes: 16000,
    max_records: 8,
  });
  expect(screen.getByText(/Ranked by distinct task terms/)).toBeTruthy();
  await user.clear(screen.getByRole("textbox", { name: "Context task phrase" }));
  await user.type(screen.getByRole("textbox", { name: "Context task phrase" }), "...?!");
  await user.click(screen.getByRole("radio", { name: "Any task terms" }));
  query.mockClear();
  await user.click(screen.getByRole("button", { name: "Build suggestions" }));
  expect((await screen.findByRole("alert")).textContent).toContain("1–8 distinct");
  expect(query).not.toHaveBeenCalled();
});

it("explains no matching candidates separately from whole records that exceed the budget", async () => {
  const f = await fixture();
  const original = f.adapter.query;
  let oversized = false;
  f.adapter.query = async (session, operation, args) => {
    const value = await original<ContextBundle>(session, operation, {
      ...args,
      query: "does-not-match-a-sample",
    });
    if (oversized) {
      value.selection.candidate_count = 2;
      value.selection.omitted_by_budget = 2;
      value.selection.selection_complete = false;
    }
    return value as never;
  };
  render(<ContextSuggestions {...f} disabled={false} onApply={() => null} onStale={() => false} />);
  const user = userEvent.setup();
  await user.click(screen.getByText("Suggest context"));
  await user.click(screen.getByRole("button", { name: "Build suggestions" }));
  expect(await screen.findByText(/No records matched this request/)).toBeTruthy();
  oversized = true;
  await user.click(screen.getByRole("button", { name: "Build suggestions" }));
  expect(await screen.findByText(/Matching records were too large/)).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Add suggestions" })).toBeNull();
});

it.each(["preset", "mode"] as const)("discards an in-flight response when the %s changes", async (control) => {
  const f = await fixture();
  const original = f.adapter.query;
  let release: (() => void) | undefined;
  let first = true;
  f.adapter.query = async (session, operation, args) => {
    if (operation === "get_context_bundle" && first) {
      first = false;
      await new Promise<void>((resolve) => { release = resolve; });
    }
    return original(session, operation, args);
  };
  const apply = vi.fn(() => null);
  render(<ContextSuggestions {...f} disabled={false} onApply={apply} onStale={() => false} />);
  const user = userEvent.setup();
  await user.click(screen.getByText("Suggest context"));
  await user.type(screen.getByRole("textbox", { name: "Context task phrase" }), "offline");
  await user.click(screen.getByRole("button", { name: "Build suggestions" }));
  await waitFor(() => expect(release).toBeTypeOf("function"));
  if (control === "preset") await user.click(screen.getByRole("button", { name: "Small" }));
  else await user.click(screen.getByRole("radio", { name: "Any task terms" }));
  await act(async () => { release?.(); });
  expect(screen.queryByText(/Retrieval result:/)).toBeNull();
  expect(screen.queryByRole("button", { name: "Add suggestions" })).toBeNull();
  expect(apply).not.toHaveBeenCalled();
  await user.click(screen.getByRole("button", { name: "Build suggestions" }));
  await screen.findByText(/Retrieval result:/);
});

it("passes Rust-supported newer Unicode task terms through frontend validation intact", async () => {
  const f = await fixture();
  const query = vi.spyOn(f.adapter, "query");
  render(<ContextSuggestions {...f} disabled={false} onApply={() => null} onStale={() => false} />);
  const user = userEvent.setup();
  await user.click(screen.getByText("Suggest context"));
  await user.click(screen.getByRole("textbox", { name: "Context task phrase" }));
  await user.paste("API\u0558/rust\u{3D000}");
  await user.click(screen.getByRole("radio", { name: "Any task terms" }));
  await user.click(screen.getByRole("button", { name: "Build suggestions" }));
  await screen.findByText(/Retrieval result:/);
  expect(screen.queryByRole("alert")).toBeNull();
  expect(query).toHaveBeenLastCalledWith(f.session, "get_context_bundle", {
    project_id: f.project.id,
    query: "API\u0558/rust\u{3D000}",
    query_mode: "terms",
    max_bytes: 16000,
    max_records: 8,
  });
});
