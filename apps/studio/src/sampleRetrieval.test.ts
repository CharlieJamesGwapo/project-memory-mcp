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

it("ranks distributed task terms before the candidate window and preserves full evidence", () => {
  const relevant = {
    ...note(999, "Preserve the complete portable paths rationale."),
    title: "Windows archive",
  };
  const newer = Array.from({ length: 205 }, (_, i) => note(i, "Windows only"));
  const bundle = sampleContextBundle(project, [...newer, relevant], {
    query: "WINDOWS paths windows",
    query_mode: "terms",
    max_records: 1,
  });
  expect(bundle.records.map((record) => record.id)).toEqual([relevant.id]);
  expect(bundle.records[0]).toMatchObject({
    ...relevant,
    selection_reason: "ranked_term_match",
    matched_terms: ["windows", "paths"],
  });
  expect(bundle.selection).toMatchObject({
    mode: "terms",
    query_terms: ["windows", "paths"],
    ranking: "term_coverage_then_headline_then_recency",
    candidate_count: 200,
    candidate_window_truncated: true,
    omitted_by_record_limit: 199,
  });
  expect(bundle.budget.used_bytes).toBeLessThanOrEqual(16000);
});

it("breaks term-coverage ties by headline coverage then newest input order", () => {
  const recentBody = note(1, "Windows paths");
  const olderHeadline = { ...note(2, "Full reasoning"), title: "Windows paths" };
  const oldestHeadline = { ...note(3, "More full reasoning"), title: "Windows paths" };
  const bundle = sampleContextBundle(project, [recentBody, olderHeadline, oldestHeadline], {
    query: "Windows paths",
    query_mode: "terms",
  });
  expect(bundle.records.map((record) => record.id)).toEqual([
    olderHeadline.id,
    oldestHeadline.id,
    recentBody.id,
  ]);
  const literal = sampleContextBundle(project, [recentBody], { query: "Windows paths" });
  expect(literal.selection.mode).toBe("query");
  expect(literal.selection).not.toHaveProperty("query_terms");
  expect(literal.records[0]).not.toHaveProperty("matched_terms");
});

it("normalizes terms only in ASCII and excludes source-only, historical and other-project matches", () => {
  const match = note(1, "Élan CAFE");
  const sourceOnly = { ...note(2, "Other text"), sources: ["Élan cafe"] };
  const historical = { ...note(3, "Élan cafe"), superseded_by: note(4).id };
  const otherProject = { ...note(5, "Élan cafe"), project_id: "different-project" };
  const bundle = sampleContextBundle(project, [sourceOnly, historical, otherProject, match], {
    query: "Élan élan CAFE cafe",
    query_mode: "terms",
  });
  expect(bundle.selection).toHaveProperty("query_terms", ["Élan", "élan", "cafe"]);
  expect(bundle.records.map((record) => record.id)).toEqual([match.id]);
  expect(bundle.records[0]).toHaveProperty("matched_terms", ["Élan", "cafe"]);
});

it("rejects unsupported or unusable term requests rather than silently changing query semantics", () => {
  for (const args of [
    { query_mode: "terms" },
    { query_mode: "terms", query: "...[]?!" },
    { query_mode: "terms", query: "one two three four five six seven eight nine" },
    { query_mode: "fuzzy", query: "Windows" },
    { query_mode: null, query: "Windows" },
    { query_mode: "terms", record_ids: [note(1).id] },
  ]) {
    expect(() => sampleContextBundle(project, [note(1)], args)).toThrow();
  }
});

it("reconsiders smaller cap-skipped sample records when final omission metadata evicts a larger one", () => {
  const first = note(1, "x".repeat(1000));
  const smaller = Array.from({ length: 10 }, (_, i) => note(i + 2, "small"));
  const records = [first, ...smaller];
  const generous = sampleContextBundle(project, records, { max_bytes: 5000, max_records: 1 });
  expect(generous.records[0].id).toBe(first.id);
  const limit = generous.budget.used_bytes - 1;
  const bounded = sampleContextBundle(project, records, { max_bytes: limit, max_records: 1 });
  expect(bounded.records.map((record) => record.id)).toEqual([smaller[0].id]);
  expect(bounded.selection).toMatchObject({
    candidate_count: 11,
    omitted_by_budget: 1,
    omitted_by_record_limit: 9,
    selection_complete: false,
  });
  expect(bounded.budget.used_bytes).toBeLessThanOrEqual(limit);
});

it("uses pinned Rust character data for newer Unicode and mixed-word sample terms", () => {
  const record = { ...note(1, "Complete reasoning"), title: "API\u0558 rust\u{3D000}" };
  const bundle = sampleContextBundle(project, [record], {
    query: "API\u0558/rust\u{3D000}",
    query_mode: "terms",
  });
  expect(bundle.selection).toHaveProperty("query_terms", ["api\u0558", "rust\u{3D000}"]);
  expect(bundle.records.map((item) => item.id)).toEqual([record.id]);
  expect(bundle.records[0]).toHaveProperty("matched_terms", ["api\u0558", "rust\u{3D000}"]);
});
