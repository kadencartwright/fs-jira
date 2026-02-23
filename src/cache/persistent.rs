use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};

use crate::jira::IssueRef;
use crate::logging;

pub type PersistentIssueRow = (String, Vec<u8>, Option<String>);
pub type PersistentSidecarRow = (String, Vec<u8>, Vec<u8>, Option<String>);

#[derive(Debug, Clone)]
/// Persisted issue markdown row.
pub struct PersistentIssue {
    pub markdown: Vec<u8>,
    pub updated: Option<String>,
}

#[derive(Debug, Clone)]
/// `tickets/index.jsonl` row persisted for fast listing.
pub struct TicketIndexRow {
    pub id: String,
    pub project: String,
    pub updated_at: Option<String>,
    pub path: String,
}

#[derive(Debug)]
/// SQLite-backed cache for issue content and sync metadata.
pub struct PersistentCache {
    conn: Mutex<Connection>,
}

impl PersistentCache {
    /// Opens or creates the persistent cache database.
    ///
    /// # Errors
    /// Returns [`rusqlite::Error`] when opening or initializing SQLite fails.
    pub fn new(path: &Path) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "
CREATE TABLE IF NOT EXISTS issues (
  issue_key TEXT PRIMARY KEY,
  markdown BLOB NOT NULL,
  updated TEXT,
  cached_at TEXT NOT NULL,
  access_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS sync_cursor (
  project TEXT PRIMARY KEY,
  last_sync TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ticket_index (
  issue_key TEXT PRIMARY KEY,
  project TEXT NOT NULL,
  updated_at TEXT,
  path TEXT NOT NULL,
  last_indexed_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ticket_index_project ON ticket_index(project);

CREATE TABLE IF NOT EXISTS issue_sidecars (
  issue_key TEXT PRIMARY KEY,
  comments_md BLOB NOT NULL,
  comments_jsonl BLOB NOT NULL,
  updated TEXT,
  cached_at TEXT NOT NULL
);

INSERT OR IGNORE INTO ticket_index(issue_key, project, updated_at, path, last_indexed_at)
SELECT
  issue_key,
  CASE
    WHEN instr(issue_key, '-') > 0 THEN substr(issue_key, 1, instr(issue_key, '-') - 1)
    ELSE 'UNKNOWN'
  END,
  updated,
  'projects/' ||
  CASE
    WHEN instr(issue_key, '-') > 0 THEN substr(issue_key, 1, instr(issue_key, '-') - 1)
    ELSE 'UNKNOWN'
  END ||
  '/' || issue_key || '.md',
  strftime('%s', 'now')
FROM issues;
 ",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Loads one persisted issue and increments its access counter.
    ///
    /// # Errors
    /// Returns [`rusqlite::Error`] when query or update execution fails.
    pub fn get_issue(&self, issue_key: &str) -> Result<Option<PersistentIssue>, rusqlite::Error> {
        let conn = lock_conn_or_recover(&self.conn);
        let mut stmt = conn.prepare("SELECT markdown, updated FROM issues WHERE issue_key = ?1")?;
        let mut rows = stmt.query(params![issue_key])?;

        if let Some(row) = rows.next()? {
            conn.execute(
                "UPDATE issues SET access_count = access_count + 1 WHERE issue_key = ?1",
                params![issue_key],
            )?;

            return Ok(Some(PersistentIssue {
                markdown: row.get(0)?,
                updated: row.get(1)?,
            }));
        }

        Ok(None)
    }

    /// Upserts one issue markdown payload.
    ///
    /// # Errors
    /// Returns [`rusqlite::Error`] when SQL execution fails.
    pub fn upsert_issue(
        &self,
        issue_key: &str,
        markdown: &[u8],
        updated: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let now = unix_epoch_seconds_string();
        let conn = lock_conn_or_recover(&self.conn);
        conn.execute(
            "
INSERT INTO issues(issue_key, markdown, updated, cached_at, access_count)
VALUES (?1, ?2, ?3, ?4, 1)
ON CONFLICT(issue_key) DO UPDATE SET
  markdown = excluded.markdown,
  updated = excluded.updated,
  cached_at = excluded.cached_at,
  access_count = issues.access_count + 1
",
            params![issue_key, markdown, updated, now],
        )?;
        upsert_ticket_index(&conn, issue_key, updated, &now)?;
        Ok(())
    }

    /// Upserts multiple issue markdown payloads in one transaction.
    ///
    /// # Errors
    /// Returns [`rusqlite::Error`] when transaction or SQL execution fails.
    pub fn upsert_issues_batch(
        &self,
        issues: &[PersistentIssueRow],
    ) -> Result<usize, rusqlite::Error> {
        let now = unix_epoch_seconds_string();
        let mut conn = lock_conn_or_recover(&self.conn);
        let tx = conn.transaction()?;

        let mut count = 0;
        for (issue_key, markdown, updated) in issues {
            tx.execute(
                "
INSERT INTO issues(issue_key, markdown, updated, cached_at, access_count)
VALUES (?1, ?2, ?3, ?4, 1)
ON CONFLICT(issue_key) DO UPDATE SET
  markdown = excluded.markdown,
  updated = excluded.updated,
  cached_at = excluded.cached_at,
  access_count = issues.access_count + 1
",
                params![issue_key, markdown, updated, now],
            )?;
            upsert_ticket_index(&tx, issue_key, updated.as_deref(), &now)?;
            count += 1;
        }

        tx.commit()?;
        Ok(count)
    }

    /// Reads the last sync cursor for a project.
    ///
    /// # Errors
    /// Returns [`rusqlite::Error`] when SQL execution fails.
    pub fn get_sync_cursor(&self, project: &str) -> Result<Option<String>, rusqlite::Error> {
        let conn = lock_conn_or_recover(&self.conn);
        let mut stmt = conn.prepare("SELECT last_sync FROM sync_cursor WHERE project = ?1")?;
        let mut rows = stmt.query(params![project])?;

        if let Some(row) = rows.next()? {
            return Ok(Some(row.get(0)?));
        }

        Ok(None)
    }

    /// Writes or updates the last sync cursor for a project.
    ///
    /// # Errors
    /// Returns [`rusqlite::Error`] when SQL execution fails.
    pub fn set_sync_cursor(&self, project: &str, last_sync: &str) -> Result<(), rusqlite::Error> {
        let conn = lock_conn_or_recover(&self.conn);
        conn.execute(
            "
INSERT INTO sync_cursor(project, last_sync)
VALUES (?1, ?2)
ON CONFLICT(project) DO UPDATE SET
  last_sync = excluded.last_sync
",
            params![project, last_sync],
        )?;
        Ok(())
    }

    /// Removes a persisted sync cursor for a project.
    ///
    /// # Errors
    /// Returns [`rusqlite::Error`] when SQL execution fails.
    pub fn clear_sync_cursor(&self, project: &str) -> Result<(), rusqlite::Error> {
        let conn = lock_conn_or_recover(&self.conn);
        conn.execute(
            "DELETE FROM sync_cursor WHERE project = ?1",
            params![project],
        )?;
        Ok(())
    }

    /// Counts persisted issues for a project key prefix.
    ///
    /// # Errors
    /// Returns [`rusqlite::Error`] when SQL execution fails.
    pub fn cached_issue_count(&self, project_prefix: &str) -> Result<usize, rusqlite::Error> {
        let conn = lock_conn_or_recover(&self.conn);
        let pattern = format!("{}-%", project_prefix);
        let count: usize = conn.query_row(
            "SELECT COUNT(*) FROM issues WHERE issue_key LIKE ?1",
            params![pattern],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Returns stored markdown size in bytes for one issue.
    ///
    /// # Errors
    /// Returns [`rusqlite::Error`] when SQL execution fails.
    pub fn issue_markdown_len(&self, issue_key: &str) -> Result<Option<u64>, rusqlite::Error> {
        let conn = lock_conn_or_recover(&self.conn);
        let mut stmt = conn.prepare("SELECT length(markdown) FROM issues WHERE issue_key = ?1")?;
        let mut rows = stmt.query(params![issue_key])?;

        if let Some(row) = rows.next()? {
            let len: i64 = row.get(0)?;
            return Ok(Some(len.max(0) as u64));
        }

        Ok(None)
    }

    /// Lists persisted ticket index rows.
    ///
    /// # Errors
    /// Returns [`rusqlite::Error`] when SQL execution fails.
    pub fn list_ticket_index(
        &self,
        projects: &[String],
    ) -> Result<Vec<TicketIndexRow>, rusqlite::Error> {
        let conn = lock_conn_or_recover(&self.conn);
        let mut stmt = conn.prepare(
            "SELECT issue_key, project, updated_at, path FROM ticket_index ORDER BY issue_key ASC",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();

        while let Some(row) = rows.next()? {
            let project: String = row.get(1)?;
            if !projects.is_empty() && !projects.iter().any(|p| p == &project) {
                continue;
            }
            out.push(TicketIndexRow {
                id: row.get(0)?,
                project,
                updated_at: row.get(2)?,
                path: row.get(3)?,
            });
        }

        Ok(out)
    }

    /// Lists issue refs for a project from the persisted ticket index.
    ///
    /// # Errors
    /// Returns [`rusqlite::Error`] when SQL execution fails.
    pub fn list_project_issue_refs(&self, project: &str) -> Result<Vec<IssueRef>, rusqlite::Error> {
        let conn = lock_conn_or_recover(&self.conn);
        let mut stmt = conn.prepare(
            "SELECT issue_key, updated_at FROM ticket_index WHERE project = ?1 ORDER BY issue_key ASC",
        )?;
        let mut rows = stmt.query(params![project])?;
        let mut out = Vec::new();

        while let Some(row) = rows.next()? {
            out.push(IssueRef {
                key: row.get(0)?,
                updated: row.get(1)?,
            });
        }

        Ok(out)
    }

    /// Upserts markdown and jsonl comment sidecars for one issue.
    ///
    /// # Errors
    /// Returns [`rusqlite::Error`] when SQL execution fails.
    pub fn upsert_issue_sidecars(
        &self,
        issue_key: &str,
        comments_md: &[u8],
        comments_jsonl: &[u8],
        updated: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let now = unix_epoch_seconds_string();
        let conn = lock_conn_or_recover(&self.conn);
        conn.execute(
            "
INSERT INTO issue_sidecars(issue_key, comments_md, comments_jsonl, updated, cached_at)
VALUES (?1, ?2, ?3, ?4, ?5)
ON CONFLICT(issue_key) DO UPDATE SET
  comments_md = excluded.comments_md,
  comments_jsonl = excluded.comments_jsonl,
  updated = excluded.updated,
  cached_at = excluded.cached_at
",
            params![issue_key, comments_md, comments_jsonl, updated, now],
        )?;
        Ok(())
    }

    /// Upserts markdown and jsonl comment sidecars in one transaction.
    ///
    /// # Errors
    /// Returns [`rusqlite::Error`] when transaction or SQL execution fails.
    pub fn upsert_issue_sidecars_batch(
        &self,
        sidecars: &[PersistentSidecarRow],
    ) -> Result<usize, rusqlite::Error> {
        let now = unix_epoch_seconds_string();
        let mut conn = lock_conn_or_recover(&self.conn);
        let tx = conn.transaction()?;

        let mut count = 0;
        for (issue_key, comments_md, comments_jsonl, updated) in sidecars {
            tx.execute(
                "
INSERT INTO issue_sidecars(issue_key, comments_md, comments_jsonl, updated, cached_at)
VALUES (?1, ?2, ?3, ?4, ?5)
ON CONFLICT(issue_key) DO UPDATE SET
  comments_md = excluded.comments_md,
  comments_jsonl = excluded.comments_jsonl,
  updated = excluded.updated,
  cached_at = excluded.cached_at
",
                params![issue_key, comments_md, comments_jsonl, updated, now],
            )?;
            count += 1;
        }

        tx.commit()?;
        Ok(count)
    }

    /// Loads markdown comment sidecar bytes for one issue.
    ///
    /// # Errors
    /// Returns [`rusqlite::Error`] when SQL execution fails.
    pub fn get_issue_comments_md(
        &self,
        issue_key: &str,
    ) -> Result<Option<Vec<u8>>, rusqlite::Error> {
        let conn = lock_conn_or_recover(&self.conn);
        let mut stmt =
            conn.prepare("SELECT comments_md FROM issue_sidecars WHERE issue_key = ?1")?;
        let mut rows = stmt.query(params![issue_key])?;
        if let Some(row) = rows.next()? {
            let bytes: Vec<u8> = row.get(0)?;
            return Ok(Some(bytes));
        }
        Ok(None)
    }

    /// Loads JSONL comment sidecar bytes for one issue.
    ///
    /// # Errors
    /// Returns [`rusqlite::Error`] when SQL execution fails.
    pub fn get_issue_comments_jsonl(
        &self,
        issue_key: &str,
    ) -> Result<Option<Vec<u8>>, rusqlite::Error> {
        let conn = lock_conn_or_recover(&self.conn);
        let mut stmt =
            conn.prepare("SELECT comments_jsonl FROM issue_sidecars WHERE issue_key = ?1")?;
        let mut rows = stmt.query(params![issue_key])?;
        if let Some(row) = rows.next()? {
            let bytes: Vec<u8> = row.get(0)?;
            return Ok(Some(bytes));
        }
        Ok(None)
    }

    /// Returns markdown sidecar size in bytes for one issue.
    ///
    /// # Errors
    /// Returns [`rusqlite::Error`] when SQL execution fails.
    pub fn issue_comments_md_len(&self, issue_key: &str) -> Result<Option<u64>, rusqlite::Error> {
        let conn = lock_conn_or_recover(&self.conn);
        let mut stmt =
            conn.prepare("SELECT length(comments_md) FROM issue_sidecars WHERE issue_key = ?1")?;
        let mut rows = stmt.query(params![issue_key])?;

        if let Some(row) = rows.next()? {
            let len: i64 = row.get(0)?;
            return Ok(Some(len.max(0) as u64));
        }

        Ok(None)
    }

    /// Returns JSONL sidecar size in bytes for one issue.
    ///
    /// # Errors
    /// Returns [`rusqlite::Error`] when SQL execution fails.
    pub fn issue_comments_jsonl_len(
        &self,
        issue_key: &str,
    ) -> Result<Option<u64>, rusqlite::Error> {
        let conn = lock_conn_or_recover(&self.conn);
        let mut stmt =
            conn.prepare("SELECT length(comments_jsonl) FROM issue_sidecars WHERE issue_key = ?1")?;
        let mut rows = stmt.query(params![issue_key])?;

        if let Some(row) = rows.next()? {
            let len: i64 = row.get(0)?;
            return Ok(Some(len.max(0) as u64));
        }

        Ok(None)
    }
}

fn lock_conn_or_recover(conn: &Mutex<Connection>) -> MutexGuard<'_, Connection> {
    match conn.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            logging::warn("recovering poisoned mutex: persistent cache connection");
            poisoned.into_inner()
        }
    }
}

fn unix_epoch_seconds_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| {
            logging::warn("system clock before unix epoch; using fallback timestamp 0");
            "0".to_string()
        })
}

fn upsert_ticket_index(
    conn: &Connection,
    issue_key: &str,
    updated: Option<&str>,
    now: &str,
) -> Result<(), rusqlite::Error> {
    let project = project_from_issue_key(issue_key);
    let path = format!("projects/{}/{}.md", project, issue_key);
    conn.execute(
        "
INSERT INTO ticket_index(issue_key, project, updated_at, path, last_indexed_at)
VALUES (?1, ?2, ?3, ?4, ?5)
ON CONFLICT(issue_key) DO UPDATE SET
  project = excluded.project,
  updated_at = excluded.updated_at,
  path = excluded.path,
  last_indexed_at = excluded.last_indexed_at
",
        params![issue_key, project, updated, path, now],
    )?;
    Ok(())
}

fn project_from_issue_key(issue_key: &str) -> String {
    issue_key
        .split_once('-')
        .map(|(project, _)| project.to_string())
        .unwrap_or_else(|| "UNKNOWN".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_and_reads_issue() {
        let db = PersistentCache::new(Path::new(":memory:")).expect("db open");
        db.upsert_issue("PROJ-1", b"hello", Some("u1"))
            .expect("upsert");

        let got = db.get_issue("PROJ-1").expect("read").expect("row present");
        assert_eq!(got.markdown, b"hello");
        assert_eq!(got.updated.as_deref(), Some("u1"));
    }

    #[test]
    fn sync_cursor_roundtrip() {
        let db = PersistentCache::new(Path::new(":memory:")).expect("db open");

        assert!(db.get_sync_cursor("PROJ").expect("get").is_none());

        db.set_sync_cursor("PROJ", "2026-02-22T10:00:00.000+0000")
            .expect("set");

        let cursor = db.get_sync_cursor("PROJ").expect("get").expect("present");
        assert_eq!(cursor, "2026-02-22T10:00:00.000+0000");

        db.set_sync_cursor("PROJ", "2026-02-22T12:00:00.000+0000")
            .expect("update");

        let cursor = db.get_sync_cursor("PROJ").expect("get").expect("present");
        assert_eq!(cursor, "2026-02-22T12:00:00.000+0000");
    }

    #[test]
    fn batch_upsert_issues() {
        let db = PersistentCache::new(Path::new(":memory:")).expect("db open");

        let issues = vec![
            (
                "PROJ-1".to_string(),
                b"content1".to_vec(),
                Some("u1".to_string()),
            ),
            (
                "PROJ-2".to_string(),
                b"content2".to_vec(),
                Some("u2".to_string()),
            ),
        ];

        let count = db.upsert_issues_batch(&issues).expect("batch upsert");
        assert_eq!(count, 2);

        let got1 = db.get_issue("PROJ-1").expect("read").expect("present");
        assert_eq!(got1.markdown, b"content1");

        let got2 = db.get_issue("PROJ-2").expect("read").expect("present");
        assert_eq!(got2.markdown, b"content2");
    }

    #[test]
    fn keeps_ticket_index_in_sync() {
        let db = PersistentCache::new(Path::new(":memory:")).expect("db open");
        db.upsert_issue("ST-10", b"v1", Some("2026-02-22T10:00:00.000+0000"))
            .expect("upsert");

        db.upsert_issue("ST-10", b"v2", Some("2026-02-22T11:00:00.000+0000"))
            .expect("upsert update");

        let rows = db
            .list_ticket_index(&["ST".to_string()])
            .expect("list ticket index");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "ST-10");
        assert_eq!(rows[0].project, "ST");
        assert_eq!(rows[0].path, "projects/ST/ST-10.md");
        assert_eq!(
            rows[0].updated_at.as_deref(),
            Some("2026-02-22T11:00:00.000+0000")
        );
    }

    #[test]
    fn persists_sidecars() {
        let db = PersistentCache::new(Path::new(":memory:")).expect("db open");
        db.upsert_issue_sidecars("DATA-1", b"md", b"jsonl", Some("u1"))
            .expect("upsert sidecars");

        let md = db
            .get_issue_comments_md("DATA-1")
            .expect("load md")
            .expect("present");
        let jsonl = db
            .get_issue_comments_jsonl("DATA-1")
            .expect("load jsonl")
            .expect("present");
        assert_eq!(md, b"md");
        assert_eq!(jsonl, b"jsonl");
        assert_eq!(
            db.issue_comments_md_len("DATA-1")
                .expect("md len")
                .expect("present"),
            2
        );
    }
}
