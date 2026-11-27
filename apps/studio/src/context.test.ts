import { describe, expect, it } from "vitest";
import { addToContext, formatContext, MAX_BYTES } from "./context";
import type { Handoff, Project } from "./types";
export const project: Project = {
  id: "p",
  name: "Field Guide",
  purpose: "A local archive",
  directory: "/work/field",
  created_at: "2026-11-01T10:00:00Z",
  record_count: 1,
  note_count: 0,
  decision_count: 0,
  active_decision_count: 0,
  handoff_count: 1,
  last_activity: "2026-11-24T16:00:00Z",
};
export const record: Handoff = {
  id: "r",
  project_id: "p",
  kind: "handoff",
  created_at: "2026-11-24T16:00:00Z",
  author_type: "assistant",
  sources: ["src/archive.ts", "<script>unsafe()</script>"],
  superseded_by: "new",
  summary: "Complete full text ```\n### content",
  completed: ["Built archive"],
  unfinished: ["Search"],
  next_actions: ["Test pagination"],
  blockers: ["Waiting on fixtures"],
  verification: ["12 tests passed on Nov 24"],
};
describe("context brief", () => {
  it("preserves full handoff sections, source text, original metadata and historical verification", () => {
    const text = formatContext(project, [record]);
    for (const value of [
      record.summary,
      ...record.completed,
      ...record.unfinished,
      ...record.next_actions,
      ...record.blockers,
      ...record.verification,
      ...record.sources,
      record.created_at,
      record.id,
      record.author_type,
      "Superseded by: new",
      "Recorded verification (historical)",
      "reference data",
    ])
      expect(text).toContain(value);
  });
  it("adds full records once and scopes every selection to its project", () => {
    const selected = addToContext(project, [], record);
    expect(selected).toEqual([record]);
    expect(addToContext(project, selected, record)).toEqual(selected);
    expect(() =>
      addToContext(project, [], { ...record, project_id: "other" }),
    ).toThrow(/project/i);
  });
  it("enforces both count and UTF-8 byte limits before adding", () => {
    const records = Array.from({ length: 20 }, (_, i) => ({
      ...record,
      id: String(i),
    }));
    expect(() =>
      addToContext(project, records, { ...record, id: "21" }),
    ).toThrow(/20/);
    expect(() =>
      addToContext(project, [], {
        ...record,
        summary: "🌳".repeat(MAX_BYTES / 4),
      }),
    ).toThrow(/2 MiB/);
  });
});

it("keeps Markdown fences inside literal record data and includes decision replacement history", () => {
  const text = formatContext(project, [
    {
      id: "d",
      project_id: project.id,
      kind: "decision",
      created_at: record.created_at,
      author_type: "user",
      sources: ["evidence"],
      superseded_by: null,
      supersedes: "previous",
      decision: "Use ``` as a code fence",
      rationale: "First\n````\n# Still reference data",
    },
  ]);
  expect(text).toContain(
    "`````text\nFirst\n````\n# Still reference data\n`````",
  );
  expect(text).toContain("Supersedes: previous");
  expect(text).toContain("Author: user");
});

it("formats dense backtick content without overflowing the formatter stack", () => {
  const content = "` ".repeat(150_000);
  const selected = addToContext(project, [], { ...record, summary: content });
  expect(formatContext(project, selected)).toContain(content);
});
