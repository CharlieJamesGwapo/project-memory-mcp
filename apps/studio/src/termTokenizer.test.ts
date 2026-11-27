import { expect, it } from "vitest";
import { taskTerms } from "./termTokenizer";

it("recognizes the Rust-supported terms rejected by the older native WebKit Unicode database", () => {
  expect(taskTerms("\u0558 \u{3D000}")).toEqual(["\u0558", "\u{3D000}"]);
});

it("keeps newer Unicode letters inside mixed task words instead of dropping or splitting them", () => {
  expect(taskTerms("API\u0558/rust\u{3D000} API\u0558")).toEqual([
    "api\u0558",
    "rust\u{3D000}",
  ]);
});

it("folds only ASCII, deduplicates in input order and treats non-alphanumeric scalars as separators", () => {
  expect(taskTerms("AUTH auth auth-2 Élan élan ① ① 🚀")).toEqual([
    "auth", "2", "Élan", "élan", "①",
  ]);
  expect(taskTerms("...[]?! 🚀")).toEqual([]);
});
