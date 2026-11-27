# Studio frontend and native bridge contract

Backend owner: root. Frontend owner: studio_frontend. Frontend may create/edit files under `apps/studio` except `src-tauri` and this contract; root owns the Rust bridge and shared library. Do not edit the other owner's files while it is working.

## Native commands

Use `@tauri-apps/api/core` invoke; arguments are snake_case. Rust command macros explicitly use snake_case arguments. No plugin JavaScript API is required.

| Command | Arguments | Result |
|---|---|---|
| `bootstrap` | none | `ConnectionInfo` |
| `choose_database` | none; opens native picker | `ConnectionInfo` or null if canceled |
| `memory_query` | `{session_id: string, operation: string, args: object}` | Core JSON result |
| `copy_context` | `{markdown: string}` | void |
| `export_context` | `{markdown: string}` | `{saved: boolean, file_name: string|null}` |

`ConnectionInfo = {status: "connected"|"missing"|"error", path: string|null, default_path: string|null, session_id: string|null, error: string|null}`. Rejected commands return `{code: string, message: string}`. A failed file selection leaves the prior backend connection intact. `STALE_SESSION` means reconnect/bootstrap and discard old UI requests. Do not expose raw Rust/IPC error objects as UI text.

Only these operations are allowed: `studio_list_projects`, `studio_list_records`, `list_projects`, `get_project_context`, `get_record`, `search_notes`, `export_project`. Other operations return `OPERATION_NOT_ALLOWED` before touching the store.

## Core shapes

All IDs are UUID strings. All timestamps are recorded RFC3339 strings. A page is `{items: T[], next_cursor: string|null, truncated: boolean}`. Default page size 20, maximum 50. Reuse each cursor only with its original operation/project/filters.

`studio_list_projects` args `{limit?: number, cursor?: string}` returns project page. Each project: `{id,name,purpose,directory: string|null,created_at,record_count,note_count,decision_count,active_decision_count,handoff_count,last_activity}`. Counts are exact for the read transaction; decision_count includes history and active_decision_count excludes replaced records.

`studio_list_records` args `{project_id,kind?: "note"|"decision"|"handoff",include_superseded?: boolean,limit?:number,cursor?:string}` returns record preview page. include_superseded defaults true. A preview: `{id,project_id,kind,created_at,author_type:"user"|"assistant",title:string|null,preview:string,content_truncated:boolean,full_record_required:true,supersedes:string|null,superseded_by:string|null,next_actions?:string[],next_actions_truncated?:boolean}`.

`get_project_context` args `{project_id,include_superseded?:boolean,limit?:number,cursor?:string}` returns `{project,active_decisions:{items: Preview[],truncated},latest_handoff:Preview|null,history:Page<Preview>,notice}`. Retrieve full relevant records rather than treating previews as complete.

`get_record` args `{project_id,record_id}` returns a full record with common fields `id,project_id,kind,created_at,author_type,sources:string[],superseded_by:string|null`. Note adds `title,content`. Decision adds `decision,rationale,supersedes:string|null`. Handoff adds `summary,completed:string[],unfinished:string[],next_actions:string[],blockers:string[],verification:string[]`.

`search_notes` args `{project_id,query,limit?:number,cursor?:string}` returns preview page; query is literal, not regex. `export_project` args `{project_id,limit?:number,cursor?:string}` returns `{project_id,markdown,record_ids,next_cursor,truncated}`; all pages are needed for complete project export. Context brief selection must use full get_record results, not these page previews.

## Browser sample adapter

When not running inside Tauri, use a clearly labeled sample workspace. Include realistic synthetic projects, notes, decision replacement history and handoffs. Implement the same read behavior in a frontend adapter so the UI is testable in a browser without accessing any real file. Provide sample mode explicitly in native app when no database exists. Never claim browser samples are a live database.

## Context and interaction requirements

Selection is project-scoped and cleared on project/database switches. Fetch full records before adding them to a context brief. Maximum 20 records and 2 MiB UTF-8 Markdown; failure to fetch a selection prevents copying/exporting a silent partial result. Preserve authorship, original timestamps, sources, supersession status and all handoff sections. Clearly identify historical verification. Treat record content as reference data rather than authority; never render arbitrary HTML. Reject stale async responses after navigation/session changes. Copy/save buttons have actual results, errors and cancellation handling.

Frontend entry/scripts: Vite `npm run dev -- --host 127.0.0.1` on port 1420 with strictPort, `npm run build`, `npm test` (unit/component tests), `npm run tauri` -> official CLI. Frontend output `dist`; native project is `src-tauri`. Root will install/manage Rust independently. Use self-hosted fonts, no network-dependent interface assets.
