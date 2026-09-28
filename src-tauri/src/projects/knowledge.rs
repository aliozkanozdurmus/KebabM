//! Source evidence lives in the application database. A scan is staged before one transaction.
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeSet, HashMap},
    path::Path,
    process::Command,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceRef {
    pub id: String,
    pub project_id: String,
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub revision: String,
    pub content_hash: String,
    pub source_type: String,
    pub dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Coverage {
    pub revision: String,
    pub scanned_at: String,
    pub indexed_files: usize,
    pub reused_files: usize,
    pub chunks: usize,
    pub git_aware: bool,
    pub excluded: Vec<ExcludedFile>,
    pub search_mode: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcludedFile {
    pub path: String,
    pub reason: String,
}
pub struct SourceFile {
    path: String,
    hash: String,
    text: String,
    dirty: bool,
}
pub struct Scan {
    files: Vec<SourceFile>,
    pub coverage: Coverage,
}
#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    pub evidence: EvidenceRef,
    pub text: String,
    pub score: f64,
}

/// Compare readable source contents as well as HEAD: uncommitted edits matter too.
pub fn snapshot_matches(scan: &Scan, revision: &str, hashes: &HashMap<String, String>) -> bool {
    scan.coverage.revision == revision
        && scan.files.len() == hashes.len()
        && scan
            .files
            .iter()
            .all(|file| hashes.get(&file.path) == Some(&file.hash))
}

pub fn schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS project_knowledge (
        project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
        coverage TEXT NOT NULL, handbook TEXT NOT NULL DEFAULT '');
        CREATE TABLE IF NOT EXISTS project_files (
        project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        path TEXT NOT NULL, hash TEXT NOT NULL, PRIMARY KEY(project_id,path));
        CREATE TABLE IF NOT EXISTS project_chunks (
        id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        path TEXT NOT NULL, start_line INTEGER NOT NULL, end_line INTEGER NOT NULL,
        revision TEXT NOT NULL, hash TEXT NOT NULL, source_type TEXT NOT NULL,
        dirty INTEGER NOT NULL, text TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS evidence_snapshots(id TEXT PRIMARY KEY, text TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS project_vectors (
        chunk_id TEXT NOT NULL REFERENCES project_chunks(id) ON DELETE CASCADE,
        model_key TEXT NOT NULL, vector TEXT NOT NULL, PRIMARY KEY(chunk_id,model_key));
        CREATE INDEX IF NOT EXISTS project_chunks_scope ON project_chunks(project_id,path);
        CREATE VIRTUAL TABLE IF NOT EXISTS project_chunks_fts USING fts5(
        path, text, content='project_chunks', content_rowid='rowid', tokenize='unicode61');
        CREATE TRIGGER IF NOT EXISTS project_chunks_ai AFTER INSERT ON project_chunks BEGIN
        INSERT INTO project_chunks_fts(rowid,path,text) VALUES(new.rowid,new.path,new.text); END;
        CREATE TRIGGER IF NOT EXISTS project_chunks_ad AFTER DELETE ON project_chunks BEGIN
        INSERT INTO project_chunks_fts(project_chunks_fts,rowid,path,text) VALUES('delete',old.rowid,old.path,old.text); END;
        CREATE TRIGGER IF NOT EXISTS project_chunks_au AFTER UPDATE ON project_chunks BEGIN
        INSERT INTO project_chunks_fts(project_chunks_fts,rowid,path,text) VALUES('delete',old.rowid,old.path,old.text);
        INSERT INTO project_chunks_fts(rowid,path,text) VALUES(new.rowid,new.path,new.text); END;")
}

fn git(root: &Path, args: &[&str]) -> Option<Vec<u8>> {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| o.stdout)
}
pub fn content_hash(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn exclusion(path: &str) -> Option<&'static str> {
    let lower = path.to_lowercase();
    if lower.split('/').any(|p| super::SKIP_DIRS.contains(&p)) {
        return Some("generated or dependency directory");
    }
    let name = lower.rsplit('/').next().unwrap_or("");
    if name.starts_with(".env")
        || ["credentials", "secrets", "id_rsa", "id_ed25519"]
            .iter()
            .any(|s| name.contains(s))
        || [".pem", ".p12", ".pfx", ".key", ".keystore"]
            .iter()
            .any(|e| name.ends_with(e))
    {
        return Some("sensitive file");
    }
    if name.ends_with(".lock")
        || name == "package-lock.json"
        || name.ends_with(".min.js")
        || name.ends_with(".map")
    {
        return Some("generated file");
    }
    let ext = name.rsplit('.').next().unwrap_or("");
    if !super::TEXT_EXT.contains(&ext)
        && ![
            "sh",
            "ps1",
            "tf",
            "hcl",
            "xml",
            "graphql",
            "proto",
            "prisma",
            "c",
            "cpp",
            "h",
            "ini",
            "cfg",
            "properties",
        ]
        .contains(&ext)
        && !name.starts_with("dockerfile.")
        && ![
            "dockerfile",
            "makefile",
            "readme",
            "jenkinsfile",
            ".gitignore",
            ".dockerignore",
            ".gitattributes",
        ]
        .contains(&name)
    {
        return Some("unsupported file type");
    }
    None
}

pub fn scan(root: &Path) -> Result<Scan, String> {
    let root = root
        .canonicalize()
        .map_err(|e| format!("Project folder unavailable: {e}"))?;
    let listed = git(
        &root,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
    );
    let git_aware = listed.is_some();
    let paths: BTreeSet<String> = if let Some(bytes) = listed {
        bytes
            .split(|b| *b == 0)
            .filter(|v| !v.is_empty())
            .map(|v| String::from_utf8_lossy(v).into_owned())
            .collect()
    } else {
        let mut paths = BTreeSet::new();
        for entry in ignore::WalkBuilder::new(&root)
            .hidden(false)
            .follow_links(false)
            .build()
        {
            let entry = entry.map_err(|e| format!("Could not enumerate project: {e}"))?;
            if entry.file_type().is_some_and(|t| t.is_file()) {
                paths.insert(
                    entry
                        .path()
                        .strip_prefix(&root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
        paths
    };
    let revision = git(&root, &["rev-parse", "HEAD"])
        .map(|v| String::from_utf8_lossy(&v).trim().to_string())
        .unwrap_or_else(|| "unversioned".into());
    let dirty: BTreeSet<String> = git(&root, &["diff", "--name-only", "-z", "HEAD"])
        .unwrap_or_default()
        .split(|b| *b == 0)
        .map(|v| String::from_utf8_lossy(v).into_owned())
        .collect();
    let tracked: BTreeSet<String> = git(&root, &["ls-files", "-z"])
        .unwrap_or_default()
        .split(|b| *b == 0)
        .map(|v| String::from_utf8_lossy(v).into_owned())
        .collect();
    let mut coverage = Coverage {
        revision,
        scanned_at: chrono::Utc::now().to_rfc3339(),
        indexed_files: 0,
        reused_files: 0,
        chunks: 0,
        git_aware,
        excluded: vec![],
        search_mode: "lexical".into(),
    };
    let mut files = Vec::new();
    for path in paths {
        let skip = |reason: &str| ExcludedFile {
            path: path.clone(),
            reason: reason.into(),
        };
        if let Some(reason) = exclusion(&path) {
            coverage.excluded.push(skip(reason));
            continue;
        }
        let candidate = root.join(&path);
        let canonical = match candidate.canonicalize() {
            Ok(p) if p.starts_with(&root) => p,
            _ => {
                coverage
                    .excluded
                    .push(skip("missing file or symlink outside project"));
                continue;
            }
        };
        let meta = canonical
            .metadata()
            .map_err(|e| format!("Cannot read metadata for {path}: {e}"))?;
        if !meta.is_file() || meta.len() > 2_000_000 {
            coverage.excluded.push(skip("not a file or exceeds 2 MB"));
            continue;
        }
        let bytes = std::fs::read(&canonical)
            .map_err(|e| format!("Cannot read {path}; previous index retained: {e}"))?;
        let text = match String::from_utf8(bytes) {
            Ok(t) if !t.contains('\0') => t,
            _ => {
                coverage.excluded.push(skip("binary or non-UTF-8"));
                continue;
            }
        };
        files.push(SourceFile {
            hash: content_hash(&text),
            dirty: dirty.contains(&path) || !tracked.contains(&path),
            path,
            text,
        });
    }
    if files.is_empty() {
        return Err("No supported sources found; previous index retained.".into());
    }
    coverage.indexed_files = files.len();
    Ok(Scan { files, coverage })
}

/// Prefer headings and declarations, but always bound chunks and retain every line.
fn chunks(text: &str) -> Vec<(usize, usize, String)> {
    let mut result = Vec::new();
    let mut body = String::new();
    let mut start = 1;
    let mut end = 0;
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        let t = line.trim_start();
        let boundary = [
            "# ",
            "## ",
            "### ",
            "pub fn ",
            "pub async fn ",
            "fn ",
            "export ",
            "function ",
            "class ",
            "def ",
            "async def ",
        ]
        .iter()
        .any(|p| t.starts_with(p));
        if !body.is_empty()
            && (n - start >= 100
                || body.len() + line.len() > 6000
                || (boundary && body.len() > 1200))
        {
            result.push((start, end, std::mem::take(&mut body)));
            start = n;
        }
        // A long single line is split at Unicode boundaries, with the same line reference.
        for c in line.chars() {
            if body.len() >= 6000 {
                result.push((start, n, std::mem::take(&mut body)));
                start = n;
            }
            body.push(c);
        }
        body.push('\n');
        end = n;
    }
    if !body.is_empty() {
        result.push((start, end, body));
    }
    result
}

pub fn commit(conn: &Connection, project: &str, mut scan: Scan) -> Result<Coverage, String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut old: HashMap<String, String> = {
        let mut q = tx
            .prepare("SELECT path,hash FROM project_files WHERE project_id=?1")
            .map_err(|e| e.to_string())?;
        let rows = q
            .query_map([project], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(|e| e.to_string())?;
        rows.collect::<rusqlite::Result<_>>()
            .map_err(|e| e.to_string())?
    };
    for file in &scan.files {
        if old.remove(&file.path).as_deref() == Some(&file.hash) {
            scan.coverage.reused_files += 1;
            tx.execute(
                "UPDATE project_chunks SET revision=?1,dirty=?2 WHERE project_id=?3 AND path=?4",
                params![scan.coverage.revision, file.dirty, project, file.path],
            )
            .map_err(|e| e.to_string())?;
            continue;
        }
        tx.execute(
            "DELETE FROM project_chunks WHERE project_id=?1 AND path=?2",
            params![project, file.path],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT OR REPLACE INTO project_files VALUES (?1,?2,?3)",
            params![project, file.path, file.hash],
        )
        .map_err(|e| e.to_string())?;
        let kind = if file.path.ends_with(".md") || file.path.ends_with(".txt") {
            "document"
        } else {
            "code"
        };
        for (start, end, text) in chunks(&file.text) {
            let id = content_hash(&format!("{project}:{}:{start}:{end}:{text}", file.path));
            tx.execute(
                "INSERT OR IGNORE INTO evidence_snapshots VALUES (?1,?2)",
                params![id, text],
            )
            .map_err(|e| e.to_string())?;
            tx.execute(
                "INSERT INTO project_chunks VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![
                    id,
                    project,
                    file.path,
                    start as i64,
                    end as i64,
                    scan.coverage.revision,
                    file.hash,
                    kind,
                    file.dirty,
                    text
                ],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    for path in old.keys() {
        tx.execute(
            "DELETE FROM project_chunks WHERE project_id=?1 AND path=?2",
            params![project, path],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM project_files WHERE project_id=?1 AND path=?2",
            params![project, path],
        )
        .map_err(|e| e.to_string())?;
    }
    scan.coverage.chunks = tx
        .query_row(
            "SELECT COUNT(*) FROM project_chunks WHERE project_id=?1",
            [project],
            |r| crate::db::row_size(r, 0),
        )
        .map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO project_knowledge(project_id,coverage) VALUES (?1,?2) ON CONFLICT(project_id) DO UPDATE SET coverage=excluded.coverage",
        params![project,serde_json::to_string(&scan.coverage).map_err(|e| e.to_string())?]).map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE projects SET file_count=?1,scanned_at=?2 WHERE id=?3",
        params![
            scan.coverage.indexed_files as i64,
            scan.coverage.scanned_at,
            project
        ],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(scan.coverage)
}

pub fn search(
    conn: &Connection,
    project: &str,
    question: &str,
    limit: usize,
) -> Result<Vec<Hit>, String> {
    let mut words: BTreeSet<String> = question
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| w.len() > 1)
        .take(40)
        .map(|w| w.to_lowercase())
        .collect();
    // Domain-neutral bilingual terms improve lexical fallback when embeddings are offline.
    for (tr, en) in [
        ("nasıl", "flow"),
        ("hata", "error"),
        ("dağıtım", "deploy"),
        ("akış", "pipeline"),
        ("belge", "document"),
        ("kaynak", "source"),
    ] {
        if question.to_lowercase().contains(tr) {
            words.insert(en.into());
        }
    }
    for stop in [
        "the", "this", "that", "how", "does", "what", "can", "you", "bir", "için", "nasıl", "ne",
        "mi", "mı", "mu", "mü", "ve", "ile", "hangi", "olur", "and", "or", "is", "are", "to", "of",
        "in",
    ] {
        words.remove(stop);
    }
    if words.is_empty() {
        return Ok(vec![]);
    }
    let query = words
        .into_iter()
        .map(|w| format!("\"{w}\""))
        .collect::<Vec<_>>()
        .join(" OR ");
    let mut stmt = conn.prepare("SELECT c.id,c.project_id,c.path,c.start_line,c.end_line,c.revision,c.hash,c.source_type,c.dirty,c.text,bm25(project_chunks_fts,1.0,1.0)
        FROM project_chunks_fts JOIN project_chunks c ON c.rowid=project_chunks_fts.rowid
        WHERE project_chunks_fts MATCH ?1 AND c.project_id=?2 ORDER BY bm25(project_chunks_fts,1.0,1.0) LIMIT 160").map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![query, project], |r| {
            Ok(Hit {
                evidence: EvidenceRef {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    path: r.get(2)?,
                    start_line: crate::db::row_size(r, 3)?,
                    end_line: crate::db::row_size(r, 4)?,
                    revision: r.get(5)?,
                    content_hash: r.get(6)?,
                    source_type: r.get(7)?,
                    dirty: r.get(8)?,
                },
                text: r.get(9)?,
                score: r.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut hits = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    let lower = question.to_lowercase();
    let test_intent = ["test", "spec", "fixture", "assert"]
        .iter()
        .any(|s| lower.contains(s));
    for hit in &mut hits {
        let path = hit.evidence.path.to_lowercase();
        // Explanatory documents and implementation both remain eligible. Tests and
        // dated archives should not crowd out the maintained explanation by filename alone.
        let is_test = path
            .split('/')
            .any(|p| ["tests", "__tests__", "fixtures"].contains(&p))
            || path.contains(".test.")
            || path.contains(".spec.");
        let agent_note = path
            .split('/')
            .any(|p| p.starts_with('.') && ![".github", ".gitlab"].contains(&p));
        let historical = path
            .split('/')
            .any(|p| ["archive", "archives", "emails"].contains(&p))
            || path
                .as_bytes()
                .windows(8)
                .any(|w| w.iter().all(u8::is_ascii_digit));
        let weight = if is_test && !test_intent {
            0.55
        } else if agent_note {
            0.55
        } else if historical {
            0.8
        } else if hit.evidence.source_type == "document" {
            1.35
        } else {
            1.0
        };
        hit.score *= weight;
    }
    hits.sort_by(|a, b| {
        a.score
            .total_cmp(&b.score)
            .then(a.evidence.path.cmp(&b.evidence.path))
            .then(a.evidence.start_line.cmp(&b.evidence.start_line))
    });
    hits.truncate(limit.min(100));
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn secret_filter_keeps_pipeline() {
        assert!(exclusion(".github/workflows/deploy.yml").is_none());
        for p in [
            ".env.production",
            "config/credentials.json",
            "certs/server.key",
        ] {
            assert!(exclusion(p).is_some());
        }
    }
    #[test]
    fn long_unicode_source_is_never_truncated() {
        let text = "ışık".repeat(5000);
        assert_eq!(
            chunks(&text)
                .iter()
                .map(|(_, _, s)| s.trim_end_matches('\n'))
                .collect::<String>(),
            text
        );
        assert!(chunks(&text).iter().all(|(s, e, _)| *s == 1 && *e == 1));
    }
    #[test]
    fn incremental_index_is_scoped_and_keeps_old_snapshot_on_failure() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrations::run(&conn).unwrap();
        let p = super::super::create(&conn, "First", "/tmp/one").unwrap();
        let other = super::super::create(&conn, "Other", "/tmp/two").unwrap();
        let make = || Scan {
            files: vec![SourceFile {
                path: ".github/workflows/deploy.yml".into(),
                hash: "hash".into(),
                text: "pipeline deploy runs cargo test".into(),
                dirty: false,
            }],
            coverage: Coverage {
                revision: "rev".into(),
                scanned_at: "now".into(),
                indexed_files: 1,
                reused_files: 0,
                chunks: 0,
                git_aware: true,
                excluded: vec![],
                search_mode: "lexical".into(),
            },
        };
        commit(&conn, &p.id, make()).unwrap();
        let hits = search(&conn, &p.id, "pipeline nasıl çalışıyor", 8).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].evidence.start_line, 1);
        assert!(search(&conn, &other.id, "pipeline", 8).unwrap().is_empty());
        assert_eq!(commit(&conn, &p.id, make()).unwrap().reused_files, 1);
        conn.execute_batch("CREATE TRIGGER fail_update BEFORE UPDATE ON projects BEGIN SELECT RAISE(ABORT,'failure'); END;").unwrap();
        let mut failed = make();
        failed.files[0].text = "other material".into();
        failed.files[0].hash = "new".into();
        assert!(commit(&conn, &p.id, failed).is_err());
        assert_eq!(search(&conn, &p.id, "pipeline", 8).unwrap().len(), 1);
    }
    #[test]
    fn freshness_detects_edits_deletions_additions_and_revision_changes() {
        let mut scan = Scan {
            files: vec![SourceFile {
                path: "README.md".into(),
                hash: "a".into(),
                text: "".into(),
                dirty: false,
            }],
            coverage: Coverage {
                revision: "rev".into(),
                scanned_at: "now".into(),
                indexed_files: 1,
                reused_files: 0,
                chunks: 0,
                git_aware: true,
                excluded: vec![],
                search_mode: "lexical".into(),
            },
        };
        let hashes = HashMap::from([("README.md".into(), "a".into())]);
        assert!(snapshot_matches(&scan, "rev", &hashes));
        assert!(!snapshot_matches(&scan, "new-rev", &hashes));
        scan.files[0].hash = "edited".into();
        assert!(!snapshot_matches(&scan, "rev", &hashes));
        scan.files.clear();
        assert!(!snapshot_matches(&scan, "rev", &hashes));
        scan.files.push(SourceFile {
            path: "new.md".into(),
            hash: "a".into(),
            text: "".into(),
            dirty: true,
        });
        assert!(!snapshot_matches(&scan, "rev", &hashes));
    }
}

/// Reciprocal-rank fusion combines ranks rather than presenting scores as confidence.
pub fn hybrid_search(
    conn: &Connection,
    project: &str,
    question: &str,
    model_key: &str,
    vector: Option<&[f32]>,
    limit: usize,
) -> Result<Vec<Hit>, String> {
    let lexical = search(conn, project, question, 40)?;
    let mut ranked: HashMap<String, (f64, Hit)> = HashMap::new();
    for (rank, hit) in lexical.into_iter().enumerate() {
        ranked.insert(hit.evidence.id.clone(), (1.0 / (61 + rank) as f64, hit));
    }
    if let Some(query) = vector {
        let mut stmt=conn.prepare("SELECT c.id,c.project_id,c.path,c.start_line,c.end_line,c.revision,c.hash,c.source_type,c.dirty,c.text,v.vector FROM project_chunks c JOIN project_vectors v ON c.id=v.chunk_id WHERE c.project_id=?1 AND v.model_key=?2").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map(params![project, model_key], |r| {
                Ok((
                    Hit {
                        evidence: EvidenceRef {
                            id: r.get(0)?,
                            project_id: r.get(1)?,
                            path: r.get(2)?,
                            start_line: crate::db::row_size(r, 3)?,
                            end_line: crate::db::row_size(r, 4)?,
                            revision: r.get(5)?,
                            content_hash: r.get(6)?,
                            source_type: r.get(7)?,
                            dirty: r.get(8)?,
                        },
                        text: r.get(9)?,
                        score: 0.0,
                    },
                    r.get::<_, String>(10)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut semantic = Vec::new();
        for row in rows {
            let (mut hit, encoded) = row.map_err(|e| e.to_string())?;
            if let Ok(values) = serde_json::from_str::<Vec<f32>>(&encoded) {
                hit.score = super::embedding::cosine(query, &values);
                if hit.score > 0.0 {
                    semantic.push(hit);
                }
            }
        }
        semantic.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then(a.evidence.id.cmp(&b.evidence.id))
        });
        for (rank, hit) in semantic.into_iter().take(40).enumerate() {
            let entry = ranked.entry(hit.evidence.id.clone()).or_insert((0.0, hit));
            entry.0 += 1.0 / (61 + rank) as f64;
        }
    }
    let mut fused: Vec<_> = ranked.into_values().collect();
    fused.sort_by(|a, b| {
        b.0.total_cmp(&a.0)
            .then(a.1.evidence.path.cmp(&b.1.evidence.path))
            .then(a.1.evidence.start_line.cmp(&b.1.evidence.start_line))
    });
    let mut seen = BTreeSet::new();
    Ok(fused
        .into_iter()
        .filter_map(|(score, mut hit)| {
            hit.score = score;
            seen.insert(content_hash(&hit.text)).then_some(hit)
        })
        .take(limit)
        .collect())
}

// A failed or cancelled indexing task always releases its project slot.
static JOBS: std::sync::OnceLock<std::sync::Mutex<BTreeSet<String>>> = std::sync::OnceLock::new();
pub struct IndexLease(String);
impl IndexLease {
    pub fn acquire(project: &str) -> Result<Self, String> {
        let mut jobs = JOBS
            .get_or_init(Default::default)
            .lock()
            .map_err(|e| e.to_string())?;
        if !jobs.insert(project.into()) {
            return Err("This project's index is already updating".into());
        }
        Ok(Self(project.into()))
    }
}
impl Drop for IndexLease {
    fn drop(&mut self) {
        if let Ok(mut jobs) = JOBS.get_or_init(Default::default).lock() {
            jobs.remove(&self.0);
        }
    }
}

/// Hash-stable chunks for approved memory and explicitly attached documents.
pub fn upsert_record(
    conn: &Connection,
    project: &str,
    path: &str,
    revision: &str,
    kind: &str,
    text: &str,
) -> Result<(), String> {
    let hash = content_hash(text);
    let same:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM project_chunks WHERE project_id=?1 AND path=?2 AND hash=?3)",params![project,path,hash],|r|r.get(0)).map_err(|e|e.to_string())?;
    if same {
        return Ok(());
    }
    conn.execute(
        "DELETE FROM project_chunks WHERE project_id=?1 AND path=?2",
        params![project, path],
    )
    .map_err(|e| e.to_string())?;
    for (start, end, body) in chunks(text) {
        let id = content_hash(&format!("{project}:{path}:{start}:{end}:{body}"));
        conn.execute(
            "INSERT OR IGNORE INTO evidence_snapshots VALUES(?1,?2)",
            params![id, body],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO project_chunks VALUES(?1,?2,?3,?4,?5,?6,?7,?8,0,?9)",
            params![
                id,
                project,
                path,
                start as i64,
                end as i64,
                revision,
                hash,
                kind,
                body
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
