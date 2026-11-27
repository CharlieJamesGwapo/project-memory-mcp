use super::{Cursor, SqlValue, Store};
use crate::{
    error::Result,
    input::{self, ListProjects, StudioRecords},
};
use serde_json::{Value, json};

impl Store {
    /// Studio-only read operation; not registered as an additional MCP tool.
    pub(super) fn studio_list_projects(&self, p: ListProjects) -> Result<Value> {
        let mut page = self.list_projects(p)?;
        if let Some(items) = page["items"].as_array_mut() {
            for project in items {
                let id = project["id"]
                    .as_str()
                    .expect("project ID comes from validated store");
                let statistics = self.conn.query_row(
                    "SELECT COUNT(*),
                       COALESCE(SUM(kind='note'),0), COALESCE(SUM(kind='decision'),0),
                       COALESCE(SUM(kind='handoff'),0),
                       (SELECT COUNT(*) FROM records a WHERE a.project_id=?1 AND a.kind='decision'
                         AND NOT EXISTS(SELECT 1 FROM records s WHERE s.supersedes=a.id)),
                       (SELECT created_at FROM records WHERE project_id=?1 ORDER BY seq DESC LIMIT 1)
                     FROM records WHERE project_id=?1",
                    [id], |r| Ok(json!({
                        "record_count":r.get::<_,i64>(0)?, "note_count":r.get::<_,i64>(1)?,
                        "decision_count":r.get::<_,i64>(2)?, "handoff_count":r.get::<_,i64>(3)?,
                        "active_decision_count":r.get::<_,i64>(4)?, "last_activity":r.get::<_,Option<String>>(5)?
                    }))
                )?;
                let fallback = project["created_at"].clone();
                for (key, value) in statistics.as_object().expect("statistics is an object") {
                    project[key] = if key == "last_activity" && value.is_null() {
                        fallback.clone()
                    } else {
                        value.clone()
                    };
                }
            }
        }
        Ok(page)
    }

    pub(super) fn studio_list_records(&self, p: StudioRecords) -> Result<Value> {
        input::id(&p.project_id)?;
        input::page(p.limit)?;
        self.project(&p.project_id)?;
        let scope = json!([
            "studio_list_records",
            p.project_id,
            p.kind,
            p.include_superseded
        ])
        .to_string();
        let cursor = Cursor::start(&scope, p.cursor.as_deref(), self.high_water("records")?)?;
        let mut filter = String::from("r.project_id=?");
        let mut bindings = vec![SqlValue::Text(p.project_id)];
        if let Some(kind) = p.kind {
            filter.push_str(" AND r.kind=?");
            bindings.push(SqlValue::Text(kind.as_str().into()));
        }
        if !p.include_superseded {
            filter.push_str(
                " AND NOT EXISTS(SELECT 1 FROM records s WHERE s.supersedes=r.id AND s.seq<=?)",
            );
            bindings.push(SqlValue::Integer(cursor.until));
        }
        let records = self.rows(&filter, bindings, &cursor, p.limit)?;
        Ok(Self::page_records(&records, p.limit, &cursor))
    }
}
