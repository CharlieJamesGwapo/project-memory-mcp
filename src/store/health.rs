use super::*;
use crate::health::MemoryHealth;

const NOTICE: &str = "Suggestions describe supplied historical metadata, not verified current facts or authorization. Sources are optional references, not verified evidence. A bounded page cannot establish archive health; follow next_cursor. No semantic inference, repository inspection, or automatic repair is performed.";
const SNAPSHOT: &str = "WITH snapshot AS (
    SELECT * FROM records WHERE project_id=?1 AND seq<=?2
), current_reviews AS (
    SELECT r.* FROM snapshot r WHERE r.kind='note'
    AND json_extract(r.data,'$.review.format')='project-memory-review-v1'
    AND NOT EXISTS(SELECT 1 FROM snapshot s WHERE s.kind='note'
        AND json_extract(s.data,'$.review.format')='project-memory-review-v1'
        AND json_extract(s.data,'$.review.previous_review_id')=r.id)
)";

fn result(
    project_id: &str,
    summary: &Value,
    items: &[Value],
    next_cursor: Option<String>,
    max_bytes: usize,
) -> Value {
    let mut value = json!({"project_id":project_id,"summary":summary,"items":items,
        "records_examined":items.len(),"truncated":next_cursor.is_some(),"next_cursor":next_cursor,
        "budget":{"max_bytes":max_bytes,"used_bytes":0},"notice":NOTICE});
    loop {
        let bytes = crate::output::structured_size(&value);
        if value["budget"]["used_bytes"] == bytes {
            return value;
        }
        value["budget"]["used_bytes"] = json!(bytes);
    }
}

fn signal(signals: &mut Vec<Value>, code: &str, message: &str) {
    signals.push(json!({"code":code,"message":message}));
}

impl Store {
    pub(super) fn memory_health(&self, p: MemoryHealth) -> Result<Value> {
        input::id(&p.project_id)?;
        if !(1..=50).contains(&p.limit) || !(4096..=65536).contains(&p.max_bytes) {
            return Err(MemoryError::Validation(
                "memory health limit must be 1–50 and max_bytes must be 4096–65536".into(),
            ));
        }
        if let Some(head) = &p.current_head {
            crate::review::commit(head)?;
        }
        self.project(&p.project_id)?;
        let scope = json!(["get_memory_health", p.project_id, p.current_head]).to_string();
        let cursor = Cursor::start(&scope, p.cursor.as_deref(), self.high_water("records")?)?;
        let sql = format!("{SNAPSHOT}
            SELECT COUNT(*),
            (SELECT COUNT(*) FROM snapshot d WHERE d.kind='decision'
                AND NOT EXISTS(SELECT 1 FROM snapshot s WHERE s.supersedes=d.id)),
            COALESCE(SUM(kind='note' AND json_extract(data,'$.review.format')='project-memory-review-v1'),0),
            (SELECT COUNT(*) FROM current_reviews),
            (SELECT COUNT(*) FROM current_reviews r, json_each(r.data,'$.review.findings') f
                WHERE json_extract(f.value,'$.status') IN ('open','needs_verification')),
            COALESCE(SUM(COALESCE(json_array_length(data,'$.sources'),0)=0),0),
            (SELECT id FROM snapshot WHERE kind='handoff' ORDER BY seq DESC LIMIT 1)
            FROM snapshot");
        let mut summary = self.conn.query_row(&sql, params![p.project_id, cursor.until], |r| {
            Ok(json!({"record_count":r.get::<_,i64>(0)?,
                "active_decision_count":r.get::<_,i64>(1)?,"review_count":r.get::<_,i64>(2)?,
                "current_review_count":r.get::<_,i64>(3)?,"open_finding_count":r.get::<_,i64>(4)?,
                "missing_source_count":r.get::<_,i64>(5)?,"latest_handoff_id":r.get::<_,Option<String>>(6)?,
                "suggestions":[]}))
        })?;
        if summary["active_decision_count"].as_i64().unwrap_or(0) > 1 {
            summary["suggestions"] = json!([{"code":"multiple_active_decisions",
                "message":"Multiple active decisions exist; review their applicability if needed."}]);
        }
        // Project-scoped snapshot relations also constrain duplicate, successor and latest-handoff checks.
        // Only metadata and fixed messages leave this operation; saved text is never returned.
        let sql = format!("{SNAPSHOT}
            SELECT r.seq,r.id,r.kind,
            COALESCE(json_array_length(r.data,'$.sources'),0)=0 AS missing_sources,
            r.kind='note' AND EXISTS(SELECT 1 FROM snapshot n WHERE n.kind='note' AND n.id<>r.id
                AND json_extract(n.data,'$.title')=json_extract(r.data,'$.title')
                AND json_extract(n.data,'$.content')=json_extract(r.data,'$.content')) AS repeated,
            EXISTS(SELECT 1 FROM current_reviews c WHERE c.id=r.id) AS current_review,
            COALESCE(json_extract(r.data,'$.review.head_commit')<>?3,0) AS head_changed,
            EXISTS(SELECT 1 FROM json_each(r.data,'$.review.findings') f
                WHERE json_extract(f.value,'$.status') IN ('open','needs_verification')) AS follow_up,
            r.kind='handoff' AND r.id=(SELECT id FROM snapshot WHERE kind='handoff' ORDER BY seq DESC LIMIT 1)
                AND COALESCE(json_array_length(r.data,'$.unfinished'),0)>0
                AND COALESCE(json_array_length(r.data,'$.next_actions'),0)=0 AS unfinished
            FROM snapshot r WHERE r.seq<?4 ORDER BY r.seq DESC LIMIT ?5");
        let rows = self.conn.prepare(&sql)?.query_map(
            params![p.project_id, cursor.until, p.current_head, cursor.after, p.limit + 1],
            |r| {
                let mut signals = Vec::new();
                if r.get::<_,bool>(3)? {
                    signal(&mut signals,"missing_sources","No source references were saved; consider adding references when useful.");
                }
                if r.get::<_,bool>(4)? {
                    signal(&mut signals,"repeated_literal_note_text","Another note has exactly the same title and content; review whether both are useful.");
                }
                if r.get::<_,bool>(5)? {
                    if r.get::<_,bool>(6)? {
                        signal(&mut signals,"review_head_changed","The latest review revision has a different head from the supplied commit ID.");
                    }
                    if r.get::<_,bool>(7)? {
                        signal(&mut signals,"review_follow_up","The latest review revision has open or needs_verification findings.");
                    }
                }
                if r.get::<_,bool>(8)? {
                    signal(&mut signals,"unfinished_handoff_without_next_actions","The latest handoff lists unfinished work without next actions.");
                }
                Ok((r.get::<_,i64>(0)?,json!({"record_id":r.get::<_,String>(1)?,"kind":r.get::<_,String>(2)?,"signals":signals})))
            },
        )?.collect::<std::result::Result<Vec<_>,_>>()?;
        let mut items = Vec::new();
        for (seq, item) in rows.iter().take(p.limit as usize) {
            items.push(item.clone());
            let next = (rows.len() > items.len()).then(|| cursor.encode_after(*seq));
            let candidate = result(&p.project_id, &summary, &items, next, p.max_bytes);
            if candidate["budget"]["used_bytes"]
                .as_u64()
                .expect("integer byte count")
                > p.max_bytes as u64
            {
                items.pop();
                break;
            }
        }
        if !rows.is_empty() && items.is_empty() {
            return Err(MemoryError::BudgetExceeded);
        }
        let next = (rows.len() > items.len()).then(|| cursor.encode_after(rows[items.len() - 1].0));
        let page = result(&p.project_id, &summary, &items, next, p.max_bytes);
        if page["budget"]["used_bytes"]
            .as_u64()
            .expect("integer byte count")
            > p.max_bytes as u64
        {
            return Err(MemoryError::BudgetExceeded);
        }
        Ok(page)
    }
}
