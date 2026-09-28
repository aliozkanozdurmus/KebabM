//! Reviewed meeting memory and preparation use the same evidence index as live help.
use super::knowledge::{self, EvidenceRef, Hit};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

pub fn schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS project_questions (
        id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        meeting_id TEXT, question TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'open',
        evidence TEXT NOT NULL DEFAULT '[]', updated_at TEXT NOT NULL,
        UNIQUE(project_id,question));
        CREATE TABLE IF NOT EXISTS project_decisions (
        id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        meeting_id TEXT, text TEXT NOT NULL, evidence TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'draft',
        updated_at TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS project_preparations (
        project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        kind TEXT NOT NULL, text TEXT NOT NULL, evidence TEXT NOT NULL, revision TEXT NOT NULL,
        created_at TEXT NOT NULL, PRIMARY KEY(project_id,kind));
        CREATE TABLE IF NOT EXISTS project_documents (
        project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        resource_id TEXT NOT NULL REFERENCES context_resources(id) ON DELETE CASCADE,
        PRIMARY KEY(project_id,resource_id));")
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryItem {
    pub id: String,
    pub text: String,
    pub status: String,
    pub evidence: Vec<EvidenceRef>,
    pub updated_at: String,
}

pub fn list(conn: &Connection, project: &str, decisions: bool) -> Result<Vec<MemoryItem>, String> {
    let sql = if decisions {
        "SELECT id,text,status,evidence,updated_at FROM project_decisions WHERE project_id=?1 ORDER BY updated_at DESC"
    } else {
        "SELECT id,question,status,evidence,updated_at FROM project_questions WHERE project_id=?1 ORDER BY updated_at DESC"
    };
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([project], |r| {
            Ok(MemoryItem {
                id: r.get(0)?,
                text: r.get(1)?,
                status: r.get(2)?,
                evidence: serde_json::from_str(&r.get::<_, String>(3)?).unwrap_or_default(),
                updated_at: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<_>>()
        .map_err(|e| e.to_string())
}

pub fn save_question(
    conn: &Connection,
    project: &str,
    meeting: Option<&str>,
    text: &str,
) -> Result<(), String> {
    let question = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if question.is_empty() || question.len() > 8000 {
        return Err("Question must contain 1–8000 characters.".into());
    }
    conn.execute("INSERT INTO project_questions(id,project_id,meeting_id,question,updated_at) VALUES (?1,?2,?3,?4,?5) ON CONFLICT(project_id,question) DO UPDATE SET updated_at=excluded.updated_at",
        params![uuid::Uuid::new_v4().to_string(),project,meeting,question,chrono::Utc::now().to_rfc3339()]).map_err(|e|e.to_string())?;
    Ok(())
}

pub fn recheck(conn: &Connection, project: &str) -> Result<usize, String> {
    let questions = list(conn, project, false)?;
    let mut found = 0;
    for q in questions.into_iter().filter(|q| q.status != "resolved") {
        let refs: Vec<_> = knowledge::search(conn, project, &q.text, 8)?
            .into_iter()
            .map(|h| h.evidence)
            .collect();
        if !refs.is_empty() {
            found += 1;
        }
        conn.execute(
            "UPDATE project_questions SET evidence=?1,status=?2,updated_at=?3 WHERE id=?4",
            params![
                serde_json::to_string(&refs).map_err(|e| e.to_string())?,
                if refs.is_empty() {
                    "open"
                } else {
                    "evidence_available"
                },
                chrono::Utc::now().to_rfc3339(),
                q.id
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(found)
}

/// Sync only project-linked material; draft decisions never enter the evidence index.
pub fn sync_records(conn: &Connection, project: &str) -> Result<(), String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut records: Vec<(String, String, String, String)> = Vec::new();
    {
        let mut stmt=tx.prepare("SELECT id,title,start_time,transcript FROM meetings WHERE project_id=?1 AND end_time IS NOT NULL ORDER BY start_time DESC").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map([project], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (id, title, date, json) = row.map_err(|e| e.to_string())?;
            let value = crate::db::meetings::get_meeting(&tx, &id)
                .map(|m| m.transcript)
                .unwrap_or_else(|_| serde_json::from_str(&json).unwrap_or_default());
            let lines = value
                .as_array()
                .map(|segments| {
                    segments
                        .iter()
                        .map(|s| {
                            format!(
                                "[{} ms] {}: {}",
                                s["timestamp_ms"],
                                s["speaker"].as_str().unwrap_or("Unknown"),
                                s["text"].as_str().unwrap_or("")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default();
            if !lines.trim().is_empty() {
                records.push((format!("memory://meetings/{id}.txt"),date,"meeting".into(),format!("Meeting: {title}\nHistorical record; not proof of current deployment.\n{lines}")));
            }
        }
    }
    for d in list(&tx, project, true)?
        .into_iter()
        .filter(|d| d.status == "approved")
    {
        records.push((
            format!("memory://decisions/{}.txt", d.id),
            d.updated_at,
            "decision".into(),
            format!(
                "Reviewed decision: {}\nEvidence: {}",
                d.text,
                serde_json::to_string(&d.evidence).unwrap_or_default()
            ),
        ));
    }
    {
        let mut stmt=tx.prepare("SELECT r.id,r.name,r.loaded_at,COALESCE(d.content,(SELECT group_concat(text,char(10)) FROM (SELECT text FROM rag_chunks WHERE file_id=r.id ORDER BY chunk_index))) FROM project_documents d JOIN context_resources r ON r.id=d.resource_id WHERE d.project_id=?1 ORDER BY r.name").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map([project], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (id, name, date, text) = row.map_err(|e| e.to_string())?;
            records.push((
                format!("memory://documents/{id}/{name}"),
                date,
                "attachment".into(),
                text,
            ));
        }
    }
    let mut retained = std::collections::BTreeSet::new();
    for (path, revision, kind, text) in records {
        retained.insert(path.clone());
        knowledge::upsert_record(&tx, project, &path, &revision, &kind, &text)?;
    }
    let obsolete = {
        let mut stmt=tx.prepare("SELECT DISTINCT path FROM project_chunks WHERE project_id=?1 AND source_type IN ('meeting','decision','attachment')").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map([project], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?
    };
    for path in obsolete.into_iter().filter(|p| !retained.contains(p)) {
        tx.execute(
            "DELETE FROM project_chunks WHERE project_id=?1 AND path=?2",
            params![project, path],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())
}

pub fn validate_evidence(
    conn: &Connection,
    project: &str,
    refs: &[EvidenceRef],
) -> Result<(), String> {
    if refs.is_empty() {
        return Err("Attach at least one project source before approving a decision.".into());
    }
    for e in refs {
        let exists:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM project_chunks WHERE id=?1 AND project_id=?2 AND path=?3 AND start_line=?4 AND end_line=?5 AND hash=?6 AND revision=?7 AND source_type=?8)",params![e.id,project,e.path,e.start_line as i64,e.end_line as i64,e.content_hash,e.revision,e.source_type],|r|r.get(0)).map_err(|e|e.to_string())?;
        if !exists || e.project_id != project {
            return Err("A decision source is no longer in this project's current index. Review its evidence again.".into());
        }
    }
    Ok(())
}

pub fn changed_sources(
    conn: &Connection,
    project: &str,
    paths: &[String],
) -> Result<Vec<Hit>, String> {
    let mut hits = Vec::new();
    for path in paths.iter().take(8) {
        let mut stmt=conn.prepare("SELECT id,project_id,path,start_line,end_line,revision,hash,source_type,dirty,text FROM project_chunks WHERE project_id=?1 AND path=?2 ORDER BY start_line LIMIT 1").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map(params![project, path], |r| {
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
                    score: 0.0,
                })
            })
            .map_err(|e| e.to_string())?;
        hits.extend(
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| e.to_string())?,
        );
    }
    Ok(hits)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionCandidate {
    pub text: String,
    pub sources: Vec<usize>,
}

/// Model output can create drafts only. Every citation must resolve to supplied meeting evidence.
pub fn save_candidates(
    conn: &Connection,
    project: &str,
    meeting: &str,
    text: &str,
    sources: &[EvidenceRef],
) -> Result<usize, String> {
    let candidates: Vec<DecisionCandidate> = serde_json::from_str(text.trim()).map_err(|_| {
        "The model returned invalid decision drafts. Retry; nothing was saved.".to_string()
    })?;
    if candidates.len() > 20 {
        return Err("Too many decision drafts; nothing was saved.".into());
    }
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut count = 0;
    for candidate in candidates {
        if candidate.text.trim().is_empty() || candidate.text.len() > 12000 {
            return Err("Invalid decision text; nothing was saved.".into());
        }
        let refs: Vec<_> = candidate
            .sources
            .into_iter()
            .map(|i| {
                i.checked_sub(1)
                    .and_then(|i| sources.get(i))
                    .cloned()
                    .ok_or("A draft cited an unknown source; nothing was saved.".to_string())
            })
            .collect::<Result<_, _>>()?;
        validate_evidence(&tx, project, &refs)?;
        let exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM project_decisions WHERE project_id=?1 AND meeting_id=?2 AND text=?3)",params![project,meeting,candidate.text.trim()],|r|r.get(0)).map_err(|e|e.to_string())?;
        if !exists {
            tx.execute("INSERT INTO project_decisions(id,project_id,meeting_id,text,evidence,updated_at) VALUES(?1,?2,?3,?4,?5,?6)",params![uuid::Uuid::new_v4().to_string(),project,meeting,candidate.text.trim(),serde_json::to_string(&refs).map_err(|e|e.to_string())?,chrono::Utc::now().to_rfc3339()]).map_err(|e|e.to_string())?;
            count += 1;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drafts_require_valid_sources_and_approval_before_indexing() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrations::run(&conn).unwrap();
        let project = crate::projects::create(&conn, "Test", "/tmp/test").unwrap();
        knowledge::upsert_record(
            &conn,
            &project.id,
            "memory://meetings/m.txt",
            "today",
            "meeting",
            "We decided to keep the Fabric limit unchanged.",
        )
        .unwrap();
        let refs = knowledge::search(&conn, &project.id, "Fabric", 8)
            .unwrap()
            .into_iter()
            .map(|h| h.evidence)
            .collect::<Vec<_>>();
        let valid = r#"[{"text":"Keep the Fabric limit unchanged.","sources":[1]}]"#;
        assert_eq!(
            save_candidates(&conn, &project.id, "m", valid, &refs).unwrap(),
            1
        );
        assert_eq!(
            save_candidates(&conn, &project.id, "m", valid, &refs).unwrap(),
            0
        );
        assert_eq!(list(&conn, &project.id, true).unwrap()[0].status, "draft");
        assert_eq!(
            conn.query_row(
                "SELECT count(*) FROM project_chunks WHERE source_type='decision'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert!(save_candidates(
            &conn,
            &project.id,
            "m",
            r#"[{"text":"first","sources":[1]},{"text":"fabricated","sources":[2]}]"#,
            &refs
        )
        .is_err());
        assert_eq!(list(&conn, &project.id, true).unwrap().len(), 1);
        let other = crate::projects::create(&conn, "Other", "/tmp/other").unwrap();
        assert!(validate_evidence(&conn, &other.id, &refs).is_err());
    }
}
