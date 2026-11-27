import { expect, it } from "vitest";
import { matchesHumanText, sampleContextBundle } from "./sampleRetrieval";
import type { Note, Project } from "./types";
const project: Project = {
  id: "58ca7fdb-e9b3-4771-a8ee-333937294101",
  name: "Sample",
  purpose: "Synthetic samples",
  directory: null,
  created_at: "2026-11-27T08:00:00Z",
  record_count: 2,
  note_count: 2,
  decision_count: 0,
  active_decision_count: 0,
  handoff_count: 0,
  last_activity: null,
};
function note(index: number, content = "small note"): Note {
  return {
    id: `58ca7fdb-e9b3-4771-a8ee-${String(index).padStart(12, "0")}`,
    project_id: project.id,
    kind: "note",
    title: "Example",
    content,
    sources: ["pointer-only"],
    author_type: "user",
    created_at: "2026-11-27T08:00:00Z",
    superseded_by: null,
  };
}
it("skips an oversized full record while allowing later smaller records", () => {
  const small = note(2, "Complete small text");
  const bundle = sampleContextBundle(
    project,
    [note(1, "🌳".repeat(20000)), small],
    { max_bytes: 4096 },
  );
  expect(bundle.records.map((record) => record.id)).toEqual([small.id]);
  expect(bundle.records[0]).toMatchObject(small);
  expect(bundle.selection.omitted_by_budget).toBe(1);
  expect(bundle.budget.used_bytes).toBeLessThanOrEqual(4096);
});
it("limits the automatic candidate window and reports every omission", () => {
  const bundle = sampleContextBundle(
    project,
    Array.from({ length: 205 }, (_, i) => note(i)),
    { max_records: 1 },
  );
  expect(bundle.selection.candidate_count).toBe(200);
  expect(bundle.selection.candidate_window_truncated).toBe(true);
  expect(bundle.selection.omitted_by_record_limit).toBe(199);
  expect(bundle.selection.selection_complete).toBe(false);
});
it("resolves explicit records atomically in request order and rejects invalid budgets and cross-project IDs", () => {
  const first = note(1),
    second = note(2);
  expect(
    sampleContextBundle(project, [first, second], {
      record_ids: [second.id, first.id],
    }).records.map((record) => record.id),
  ).toEqual([second.id, first.id]);
  expect(() =>
    sampleContextBundle(project, [first], { record_ids: [second.id] }),
  ).toThrow();
  expect(() =>
    sampleContextBundle(project, [first], {
      query: "test",
      record_ids: [first.id],
    }),
  ).toThrow();
  expect(() =>
    sampleContextBundle(project, [note(1, "x".repeat(9000))], {
      record_ids: [first.id],
      max_bytes: 4096,
    }),
  ).toThrow();
  expect(() =>
    sampleContextBundle(project, [first], { max_bytes: 4000 }),
  ).toThrow();
});
it("matches ASCII case-insensitively and other Unicode exactly without searching source pointers", () => {
  const record = note(1, "ASCII Élan [literal] newline\nNUL\0value");
  expect(matchesHumanText(record, "ascii")).toBe(true);
  expect(matchesHumanText(record, "élan")).toBe(false);
  expect(matchesHumanText(record, "Élan")).toBe(true);
  expect(matchesHumanText(record, "[literal]")).toBe(true);
  expect(matchesHumanText(record, "\0")).toBe(true);
  expect(matchesHumanText(record, "pointer-only")).toBe(false);
});
