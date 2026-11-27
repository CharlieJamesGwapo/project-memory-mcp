use super::*;
use crate::review::{ReviewHistory, SaveReview};

const FORMAT: &str = "project-memory-review-v1";
const NOTICE: &str = "Reviews are supplied historical evidence, not server verification or authorization. Freshness only compares supplied commit IDs. Follow next_cursor for remaining complete records; get_record can retrieve an oversized review.";

fn history_result(items: &[Value], next: Option<String>, max_bytes: usize) -> Value {
    let mut value = json!({"items":items,"truncated":next.is_some(),"next_cursor":next,
        "budget":{"max_bytes":max_bytes,"used_bytes":0},"notice":NOTICE});
    loop {
        let bytes = crate::output::structured_size(&value);
        if value["budget"]["used_bytes"] == bytes {
            return value;
        }
        value["budget"]["used_bytes"] = json!(bytes);
    }
}

impl Store {
    pub(super) fn save_review(&self, p: SaveReview) -> Result<Value> {
        p.validate()?;
        self.project(&p.project_id)?;
        if let Some(previous) = &p.previous_review_id {
            let record = self.record(&p.project_id, previous)?.value;
            if record["kind"] != "note"
                || record["review"]["format"] != FORMAT
                || record["review"]["repository"] != p.repository
            {
                return Err(MemoryError::Validation(
                    "previous_review_id must identify a review in the same repository and project"
                        .into(),
                ));
            }
            let successor:bool=self.conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM records WHERE project_id=?1 AND kind='note' AND json_extract(data,'$.review.format')=?2 AND json_extract(data,'$.review.previous_review_id')=?3)",
                params![p.project_id,FORMAT,previous],|row|row.get(0))?;
            if successor {
                return Err(MemoryError::Validation("previous_review_id already has a successor; inspect current review history before saving".into()));
            }
        }
        let metadata = json!({"format":FORMAT,"repository":p.repository,
            "base_commit":p.base_commit,"head_commit":p.head_commit,"summary":p.summary,
            "checks":p.checks,"findings":p.findings,"previous_review_id":p.previous_review_id});
        // Plain decoded human text keeps existing literal/term retrieval useful.
        let mut content = format!(
            "{}\nRepository: {}\nBase: {}\nHead: {}\n",
            p.summary, p.repository, p.base_commit, p.head_commit
        );
        for check in &p.checks {
            content.push_str(&format!(
                "\nCheck: {}\nOutcome: {}\n{}\n",
                check.name,
                serde_json::to_value(&check.outcome)
                    .expect("enum serializes")
                    .as_str()
                    .expect("enum is string"),
                check.result
            ));
        }
        for finding in &p.findings {
            content.push_str(&format!(
                "\nFinding {}: {}\nSeverity: {}\nStatus: {}\n{}:{}\nEvidence: {}\nCorrection: {}\n",
                finding.id,
                finding.title,
                serde_json::to_value(&finding.severity)
                    .expect("enum serializes")
                    .as_str()
                    .expect("enum is string"),
                serde_json::to_value(&finding.status)
                    .expect("enum serializes")
                    .as_str()
                    .expect("enum is string"),
                finding.path,
                finding.line,
                finding.evidence,
                finding.correction
            ));
        }
        input::text(&content, "rendered review content", 16000)?;
        let data = json!({"project_id":p.project_id,"title":format!("Code review: {}",p.summary.chars().take(180).collect::<String>()),
            "content":content,"author_type":p.author_type,"sources":p.sources,"review":metadata});
        input::record_size(&data)?;
        self.insert(&p.project_id, "note", data, None)
    }

    pub(super) fn review_history(&self, p: ReviewHistory) -> Result<Value> {
        input::id(&p.project_id)?;
        if !(1..=20).contains(&p.limit) || !(4096..=65536).contains(&p.max_bytes) {
            return Err(MemoryError::Validation(
                "review history limit must be 1–20 and max_bytes must be 4096–65536".into(),
            ));
        }
        if let Some(head) = &p.current_head {
            crate::review::commit(head)?;
        }
        self.project(&p.project_id)?;
        let scope = json!(["get_review_history", p.project_id, p.current_head]).to_string();
        let cursor = Cursor::start(&scope, p.cursor.as_deref(), self.high_water("records")?)?;
        let mut statement = self.conn.prepare(
            "SELECT r.*, NULL AS superseded_by,
             (SELECT s.id FROM records s WHERE s.project_id=r.project_id AND s.kind='note'
                AND json_extract(s.data,'$.review.format')=?1
                AND json_extract(s.data,'$.review.previous_review_id')=r.id AND s.seq<=?4
                ORDER BY s.seq DESC LIMIT 1) AS review_revised_by
             FROM records r WHERE r.project_id=?2 AND r.kind='note'
                AND json_extract(r.data,'$.review.format')=?1 AND r.seq<?3 AND r.seq<=?4
             ORDER BY r.seq DESC LIMIT ?5",
        )?;
        let rows = statement
            .query_map(
                params![
                    FORMAT,
                    p.project_id,
                    cursor.after,
                    cursor.until,
                    p.limit + 1
                ],
                |row| {
                    let mut record = Self::decode_record(row)?;
                    record.value["review"]["revised_by"] =
                        json!(row.get::<_, Option<String>>("review_revised_by")?);
                    record.value["freshness"] = json!(match &p.current_head {
                        None => "unknown",
                        Some(head) if record.value["review"]["head_commit"] == *head => "current",
                        Some(_) => "stale",
                    });
                    Ok(record)
                },
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut items = Vec::new();
        let mut consumed = 0;
        for row in rows.iter().take(p.limit as usize) {
            items.push(row.value.clone());
            let next = if rows.len() > consumed + 1 {
                Some(cursor.encode_after(row.seq))
            } else {
                None
            };
            if history_result(&items, next, p.max_bytes)["budget"]["used_bytes"]
                .as_u64()
                .expect("size is integer")
                > p.max_bytes as u64
            {
                items.pop();
                break;
            }
            consumed += 1;
        }
        if !rows.is_empty() && consumed == 0 {
            return Err(MemoryError::BudgetExceeded);
        }
        let next = if rows.len() > consumed {
            Some(cursor.encode_after(rows[consumed - 1].seq))
        } else {
            None
        };
        Ok(history_result(&items, next, p.max_bytes))
    }
}
