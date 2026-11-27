use super::{CANDIDATE_LIMIT, HUMAN_TEXT, with_reason};
use crate::{
    error::{MemoryError, Result},
    store::Store,
};
use rusqlite::{params_from_iter, types::Value as SqlValue};
use serde_json::{Value, json};
use std::collections::HashSet;

// Share literal retrieval's decoded human-text fields, excluding source pointers,
// IDs, authorship and JSON serialization.
const HEADLINE: &str = "t.type='text' AND t.fullkey IN ('$.title','$.decision','$.summary')";

pub(super) struct TermQuery {
    pub terms: Vec<String>,
}

impl TermQuery {
    pub fn parse(query: &str) -> Result<Self> {
        let mut seen = HashSet::new();
        let terms: Vec<_> = query
            .split(|character: char| !character.is_alphanumeric())
            .filter(|term| !term.is_empty())
            .map(str::to_ascii_lowercase)
            .filter(|term| seen.insert(term.clone()))
            .collect();
        if !(1..=8).contains(&terms.len()) {
            return Err(MemoryError::Validation(
                "terms query must contain 1–8 distinct alphanumeric terms".into(),
            ));
        }
        Ok(Self { terms })
    }

    pub fn candidates(&self, store: &Store, project: &str) -> Result<(Vec<Value>, bool)> {
        let high = store.high_water("records")?;
        // Only the number of fixed placeholders varies. Every caller value binds.
        let placeholders = std::iter::repeat_n("(?)", self.terms.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "WITH query_terms(term) AS (VALUES {placeholders}), scored AS (
                SELECT r.*,
                    (SELECT id FROM records s WHERE s.supersedes=r.id AND s.seq<=?) AS superseded_by,
                    (SELECT COUNT(*) FROM query_terms q WHERE EXISTS (
                        SELECT 1 FROM json_tree(r.data) t WHERE {HUMAN_TEXT}
                        AND instr(lower(t.value),q.term)>0
                    )) AS term_coverage,
                    (SELECT COUNT(*) FROM query_terms q WHERE EXISTS (
                        SELECT 1 FROM json_tree(r.data) t WHERE {HEADLINE}
                        AND instr(lower(t.value),q.term)>0
                    )) AS headline_coverage
                FROM records r WHERE r.project_id=? AND r.seq<=?
                    AND NOT EXISTS(SELECT 1 FROM records s WHERE s.supersedes=r.id AND s.seq<=?)
            ) SELECT * FROM scored WHERE term_coverage>0
              ORDER BY term_coverage DESC,headline_coverage DESC,seq DESC LIMIT 201"
        );
        let mut bindings: Vec<SqlValue> = self.terms.iter().cloned().map(Into::into).collect();
        bindings.extend([
            SqlValue::Integer(high),
            project.to_owned().into(),
            SqlValue::Integer(high),
            SqlValue::Integer(high),
        ]);
        let rows = store
            .conn
            .prepare(&sql)?
            .query_map(params_from_iter(bindings), Store::decode_record)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let truncated = rows.len() > CANDIDATE_LIMIT;
        let records = rows
            .into_iter()
            .take(CANDIDATE_LIMIT)
            .map(|row| {
                let mut value = with_reason(row.value, "ranked_term_match");
                value["matched_terms"] = json!(self.matched_terms(&value));
                value
            })
            .collect();
        Ok((records, truncated))
    }

    fn matched_terms<'a>(&'a self, record: &Value) -> Vec<&'a str> {
        let mut text = Vec::new();
        for field in ["title", "content", "decision", "rationale", "summary"] {
            if let Some(value) = record[field].as_str() {
                text.push(value.to_ascii_lowercase());
            }
        }
        for field in [
            "completed",
            "unfinished",
            "next_actions",
            "blockers",
            "verification",
        ] {
            if let Some(values) = record[field].as_array() {
                text.extend(
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_ascii_lowercase),
                );
            }
        }
        self.terms
            .iter()
            .filter(|term| text.iter().any(|value| value.contains(term.as_str())))
            .map(String::as_str)
            .collect()
    }
}
