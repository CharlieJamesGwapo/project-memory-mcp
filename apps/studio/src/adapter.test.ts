import { describe, expect, it } from "vitest";
import { createNativeAdapter } from "./adapter";
import { createSampleAdapter } from "./sample";
import type { MemoryRecord, Page, Preview, Project } from "./types";
describe("native adapter", () => {
  it("uses exact native commands with snake_case arguments", async () => {
    const calls: unknown[] = [];
    const invoke = async (command: string, args?: Record<string, unknown>) => {
      calls.push([command, args]);
      return { saved: false, file_name: null };
    };
    const adapter = createNativeAdapter(invoke);
    await adapter.bootstrap();
    await adapter.chooseDatabase();
    await adapter.query("session", "get_record", {
      project_id: "p",
      record_id: "r",
    });
    await adapter.copy("full");
    expect(await adapter.save("full")).toEqual({
      saved: false,
      file_name: null,
    });
    expect(calls).toEqual([
      ["bootstrap", undefined],
      ["choose_database", undefined],
      [
        "memory_query",
        {
          session_id: "session",
          operation: "get_record",
          args: { project_id: "p", record_id: "r" },
        },
      ],
      ["copy_context", { markdown: "full" }],
      ["export_context", { markdown: "full" }],
    ]);
  });
});
describe("sample workspace", () => {
  it("paginates project-scoped records, searches literal strings and retrieves complete data", async () => {
    const a = createSampleAdapter();
    const conn = await a.bootstrap();
    expect(a.sample).toBe(true);
    expect(conn.session_id).toBeTruthy();
    const projects = await a.query<Page<Project>>(
      conn.session_id!,
      "studio_list_projects",
      {},
    );
    expect(projects.items.length).toBeGreaterThan(1);
    const id = projects.items[0].id;
    const first = await a.query<Page<Preview>>(
      conn.session_id!,
      "studio_list_records",
      { project_id: id, limit: 1 },
    );
    expect(first.items).toHaveLength(1);
    expect(first.next_cursor).toBeTruthy();
    const second = await a.query<Page<Preview>>(
      conn.session_id!,
      "studio_list_records",
      { project_id: id, limit: 1, cursor: first.next_cursor },
    );
    expect(second.items[0].id).not.toBe(first.items[0].id);
    const full = await a.query<MemoryRecord>(conn.session_id!, "get_record", {
      project_id: id,
      record_id: first.items[0].id,
    });
    expect(full.sources.length).toBeGreaterThan(0);
    const literal = await a.query<Page<Preview>>(
      conn.session_id!,
      "search_notes",
      { project_id: id, query: ".*" },
    );
    expect(literal.items).toHaveLength(0);
    await expect(
      a.query(conn.session_id!, "get_record", {
        project_id: projects.items[1].id,
        record_id: full.id,
      }),
    ).rejects.toThrow();
  });
});

it("sample context and paginated project exports respect history filters and preserve full records", async () => {
  const a = createSampleAdapter();
  const conn = await a.bootstrap();
  const p = await a.query<Page<Project>>(
    conn.session_id!,
    "studio_list_projects",
    {},
  );
  const project_id = p.items[0].id;
  const context = await a.query<{ history: Page<Preview> }>(
    conn.session_id!,
    "get_project_context",
    { project_id, include_superseded: false },
  );
  expect(context.history.items.every((item) => !item.superseded_by)).toBe(true);
  const result = await a.query<{
    markdown: string;
    record_ids: string[];
    next_cursor: string | null;
    truncated: boolean;
  }>(conn.session_id!, "export_project", { project_id, limit: 1 });
  expect(result.record_ids).toHaveLength(1);
  expect(result.markdown).toContain(
    "Recorded on 26 Nov: 28 unit tests passed.",
  );
  expect(result.next_cursor).toBeTruthy();
  expect(result.truncated).toBe(true);
});
