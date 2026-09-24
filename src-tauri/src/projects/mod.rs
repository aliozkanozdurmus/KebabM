use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const SKIP_DIRS: &[&str] = &[
    ".git", "node_modules", "dist", "target", "build", ".next", "vendor",
    "__pycache__", ".venv", "venv", "coverage", ".turbo", "out", "zaiqo-meet",
];

const TEXT_EXT: &[&str] = &[
    "md", "txt", "rs", "ts", "tsx", "js", "jsx", "py", "go", "json", "toml",
    "yml", "yaml", "css", "html", "sql", "cs", "java", "kt", "vue", "svelte",
    "rb", "php", "swift",
];

#[derive(Debug, Clone, Serialize)]
pub struct ProjectRecord {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub brief: String,
    pub file_count: i64,
    pub scanned_at: Option<String>,
    pub is_active: bool,
}

pub struct ProjectPack {
    pub instructions: String,
    pub context: String,
}

pub fn create(conn: &Connection, name: &str, root_path: &str) -> Result<ProjectRecord, String> {
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO projects (id, name, root_path, brief, file_count, is_active) VALUES (?1, ?2, ?3, '', 0, 0)",
        params![id, name, root_path],
    )
    .map_err(|e| e.to_string())?;
    get(conn, &id)
}

pub fn list(conn: &Connection) -> Result<Vec<ProjectRecord>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name, root_path, brief, file_count, scanned_at, is_active FROM projects ORDER BY name")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_row)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize)]
pub struct ModuleCard {
    pub name: String,
    pub summary: String,
}

pub fn modules(conn: &Connection, id: &str) -> Result<Vec<ModuleCard>, String> {
    let mut stmt = conn
        .prepare("SELECT name, summary FROM project_modules WHERE project_id = ?1 ORDER BY name")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![id], |row| {
            Ok(ModuleCard {
                name: row.get(0)?,
                summary: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn get(conn: &Connection, id: &str) -> Result<ProjectRecord, String> {
    conn.query_row(
        "SELECT id, name, root_path, brief, file_count, scanned_at, is_active FROM projects WHERE id = ?1",
        params![id],
        map_row,
    )
    .map_err(|e| e.to_string())
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM project_excerpts WHERE project_id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM project_modules WHERE project_id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM projects WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn set_active(conn: &Connection, id: Option<&str>) -> Result<(), String> {
    conn.execute("UPDATE projects SET is_active = 0", [])
        .map_err(|e| e.to_string())?;
    if let Some(id) = id {
        conn.execute("UPDATE projects SET is_active = 1 WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn active(conn: &Connection) -> Result<Option<ProjectRecord>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name, root_path, brief, file_count, scanned_at, is_active FROM projects WHERE is_active = 1 LIMIT 1")
        .map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        Ok(Some(map_row(row).map_err(|e| e.to_string())?))
    } else {
        Ok(None)
    }
}

pub fn prepare_index(conn: &Connection, id: &str) -> Result<String, String> {
    let project = get(conn, id)?;
    let root = PathBuf::from(&project.root_path);
    if !root.is_dir() {
        return Err(format!("Project folder does not exist: {}", project.root_path));
    }

    let mut files = Vec::new();
    walk(&root, &root, &mut files);
    files.sort_by_key(|f| priority(&f.rel));
    if files.is_empty() {
        return Err("No readable source or docs were found in that folder.".to_string());
    }

    conn.execute("DELETE FROM project_excerpts WHERE project_id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM project_modules WHERE project_id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    let file_count = files.len() as i64;
    reset_knowledge(&root)?;
    let inventory = files.iter().map(|file| format!("- {}", file.rel)).collect::<Vec<_>>().join("\n");
    write_knowledge(
        &root,
        "inventory.md",
        &format!("# Files seen while mapping the project\n\n{inventory}\n"),
    )?;
    conn.execute(
        "UPDATE projects SET brief = '', file_count = ?1, scanned_at = NULL WHERE id = ?2",
        params![file_count, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(build_dossier(&files))
}

fn reset_knowledge(root: &Path) -> Result<(), String> {
    let dir = knowledge_dir(root);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())
}

pub fn knowledge_dir(root: &Path) -> PathBuf {
    root.join("zaiqo-meet")
}

pub fn write_knowledge(root: &Path, relative: &str, body: &str) -> Result<(), String> {
    let path = knowledge_dir(root).join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, body).map_err(|e| e.to_string())
}

pub fn read_knowledge(root: &Path) -> Result<Vec<(String, String)>, String> {
    let dir = knowledge_dir(root);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    collect_markdown(&dir, &dir, &mut files);
    files.sort_by_key(|(rel, _)| knowledge_rank(rel));
    Ok(files)
}

pub fn save_module(conn: &Connection, project_id: &str, name: &str, summary: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO project_modules (id, project_id, name, summary) VALUES (?1, ?2, ?3, ?4)",
        params![Uuid::new_v4().to_string(), project_id, name, summary],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub fn finish_scan(conn: &Connection, id: &str, brief: &str) -> Result<ProjectRecord, String> {
    let scanned_at = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE projects SET brief = ?1, scanned_at = ?2 WHERE id = ?3",
        params![brief, scanned_at, id],
    )
    .map_err(|e| e.to_string())?;
    get(conn, id)
}

pub fn save_brief(conn: &Connection, id: &str, brief: &str) -> Result<(), String> {
    conn.execute("UPDATE projects SET brief = ?1 WHERE id = ?2", params![brief, id])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn pack(conn: &Connection, _question: &str) -> Result<Option<ProjectPack>, String> {
    let Some(project) = active(conn)? else {
        return Ok(None);
    };
    if project.file_count == 0 && project.brief.is_empty() {
        return Ok(None);
    }

    let root = PathBuf::from(&project.root_path);
    let docs = read_knowledge(&root)?;
    let mut context = format!(
        "# Meeting knowledge base for {}\nFolder: {}\\zaiqo-meet\n\n",
        project.name, project.root_path
    );
    let mut used = context.len();
    for (path, text) in &docs {
        if path.ends_with("inventory.md") {
            continue;
        }
        if used > 48_000 {
            break;
        }
        let body = clip(text, 8000);
        let block = format!("\n# {path}\n{body}\n");
        used += block.len();
        context.push_str(&block);
    }
    if docs.is_empty() {
        context.push_str(&clip(&project.brief, 14000));
    }
    if let Ok(notes) = recent_meeting_notes(conn, &project.id) {
        if !notes.is_empty() && used < 52_000 {
            context.push_str("\n\n# Earlier meetings on this project\n");
            context.push_str(&notes);
        }
    }

    let instructions = format!(
        "You are in a meeting about \"{}\". \
         The only project source is the zaiqo-meet knowledge base below. \
         It was written by reading the repository from start to end. \
         Answer only from those notes. Cite the note or the source path named inside it. \
         If the knowledge base does not contain the fact, say it is not in zaiqo-meet. \
         When the question is about structure or flow, add one fenced mermaid diagram from those notes. \
         End with exactly three follow-up questions about this same topic. \
         They must be answerable from zaiqo-meet, not generic.\n\n\
         ## Follow-ups\n- \n- \n- ",
        project.name
    );

    Ok(Some(ProjectPack { instructions, context }))
}

fn recent_meeting_notes(conn: &Connection, project_id: &str) -> Result<String, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, title, start_time, summary FROM meetings
             WHERE project_id = ?1 AND end_time IS NOT NULL
             ORDER BY start_time DESC LIMIT 4",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![project_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut notes = String::new();
    for row in rows {
        let (id, title, start, summary) = row.map_err(|e| e.to_string())?;
        let body = match summary.filter(|s| !s.trim().is_empty()) {
            Some(summary) => clip(&summary, 1200),
            None => transcript_excerpt(conn, &id, 1200)?,
        };
        if body.is_empty() {
            continue;
        }
        notes.push_str(&format!("\n## {title} ({start})\n{body}\n"));
        if notes.len() > 6000 {
            break;
        }
    }
    Ok(notes)
}

fn transcript_excerpt(conn: &Connection, meeting_id: &str, max_chars: usize) -> Result<String, String> {
    let mut stmt = conn
        .prepare(
            "SELECT speaker, text FROM transcript_segments
             WHERE meeting_id = ?1 AND is_final = 1
             ORDER BY timestamp_ms ASC LIMIT 80",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![meeting_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?;
    let mut text = String::new();
    for row in rows {
        let (speaker, line) = row.map_err(|e| e.to_string())?;
        text.push_str(&format!("{speaker}: {line}\n"));
        if text.len() >= max_chars {
            break;
        }
    }
    Ok(clip(&text, max_chars))
}

fn build_dossier(files: &[ScannedFile]) -> String {
    const CAP: usize = 24_000;
    let mut out = String::from(
        "# Scout pack\nA code agent would open the map, the product files, and the surface of the rest. It would not read every line.\n\n## Folders\n",
    );
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for file in files {
        *counts.entry(area_name(&file.rel)).or_default() += 1;
    }
    for (area, count) in &counts {
        out.push_str(&format!("- {area}: {count} files\n"));
    }
    out.push_str("\n## Paths\n");
    for file in files {
        if out.len() > 7_000 {
            out.push_str("- Further paths stay in the folder counts above.\n");
            break;
        }
        out.push_str(&format!("- {}\n", file.rel));
    }
    out.push_str("\n## Files that explain the product\n");
    let mut full_budget = 12_000usize;
    for file in files {
        if !is_core(file) || full_budget < 300 || out.len() > CAP {
            continue;
        }
        let body = clip(&file.text, full_budget.min(3_500));
        full_budget = full_budget.saturating_sub(body.len());
        out.push_str(&format!("\n### {}\n{body}\n", file.rel));
    }
    out.push_str("\n## Surface of the other files\n");
    for file in files {
        if is_core(file) || out.len() > CAP {
            continue;
        }
        let sketch = sketch_file(&file.text);
        if sketch.is_empty() {
            continue;
        }
        out.push_str(&format!("\n### {}\n{sketch}\n", file.rel));
    }
    clip(&out, CAP)
}

fn is_core(file: &ScannedFile) -> bool {
    let lower = file.rel.to_lowercase();
    priority(&file.rel) <= 1
        || lower.ends_with("dockerfile")
        || lower.contains("schema")
        || lower.ends_with("/main.rs")
        || lower.ends_with("/main.ts")
        || lower.ends_with("/main.py")
        || lower.ends_with("/app.tsx")
        || lower.ends_with("/app.ts")
        || lower.contains("/routes/")
        || lower.contains("/route.")
}

fn sketch_file(text: &str) -> String {
    let mut lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let interesting = trimmed.starts_with('#')
            || trimmed.starts_with("export ")
            || trimmed.starts_with("pub ")
            || trimmed.starts_with("class ")
            || trimmed.starts_with("def ")
            || trimmed.starts_with("function ")
            || trimmed.starts_with("interface ")
            || trimmed.starts_with("type ")
            || trimmed.starts_with("fn ");
        if interesting || lines.is_empty() {
            lines.push(clip(trimmed, 140));
        }
        if lines.len() >= 4 {
            break;
        }
    }
    lines.join("\n")
}

fn collect_markdown(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_markdown(root, &path, out);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let rel = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().replace('\\', "/");
        out.push((rel, text));
    }
}

fn knowledge_rank(rel: &str) -> (u8, String) {
    let order = if rel.ends_with("overview.md") {
        0
    } else if rel.ends_with("architecture.md") {
        1
    } else if rel.ends_with("flows.md") {
        2
    } else if rel.ends_with("domain.md") {
        3
    } else if rel.ends_with("where-to-look.md") {
        4
    } else if rel.ends_with("meeting-guide.md") {
        5
    } else if rel.ends_with("risks.md") {
        6
    } else if rel.ends_with("inventory.md") {
        7
    } else {
        8
    };
    (order, rel.to_string())
}

fn area_name(rel: &str) -> String {
    let parts: Vec<&str> = rel.split('/').collect();
    match parts.as_slice() {
        [file] => format!("root/{file}"),
        [a, _] => (*a).to_string(),
        [a, b, ..] => format!("{a}/{b}"),
        _ => "root".to_string(),
    }
}

fn clip(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

struct ScannedFile {
    rel: String,
    text: String,
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<ScannedFile>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if SKIP_DIRS.contains(&name.as_str()) || name.starts_with('.') {
                continue;
            }
            walk(root, &path, out);
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !TEXT_EXT.contains(&ext) && !name.eq_ignore_ascii_case("readme") && !name.eq_ignore_ascii_case("dockerfile") {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(meta) => meta,
            Err(_) => continue,
        };
        if meta.len() > 200_000 || meta.len() == 0 {
            continue;
        }
        let raw = match std::fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(_) => continue,
        };
        if raw.trim().is_empty() {
            continue;
        }
        let rel = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().replace('\\', "/");
        out.push(ScannedFile { rel, text: raw });
    }
}

fn priority(rel: &str) -> u8 {
    let lower = rel.to_lowercase();
    if lower.contains("readme") { 0 }
    else if lower.ends_with("package.json") || lower.ends_with("cargo.toml") || lower.ends_with("pyproject.toml") { 1 }
    else if lower.ends_with(".md") { 2 }
    else { 3 }
}

#[cfg(test)]
mod source_batch_tests {
    use super::*;

    #[test]
    fn scout_pack_names_every_file_without_dumping_it() {
        let mut files: Vec<ScannedFile> = (0..25)
            .map(|i| ScannedFile {
                rel: format!("src/area/file{i}.ts"),
                text: "x".repeat(1000),
            })
            .collect();
        files.push(ScannedFile {
            rel: "README.md".to_string(),
            text: "HELLO PRODUCT\nThis app reviews hauler bills.".to_string(),
        });
        let dossier = build_dossier(&files);
        assert!(dossier.contains("HELLO PRODUCT"));
        assert!(dossier.contains("src/area: 25 files") || dossier.contains("src/area:"));
        assert!(dossier.contains("src/area/file0.ts"));
        assert!(!dossier.contains(&"x".repeat(500)));
        assert!(dossier.chars().count() <= 24_000);
    }
}

fn map_row(row: &rusqlite::Row) -> rusqlite::Result<ProjectRecord> {
    Ok(ProjectRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        root_path: row.get(2)?,
        brief: row.get(3)?,
        file_count: row.get(4)?,
        scanned_at: row.get(5)?,
        is_active: row.get::<_, i64>(6)? != 0,
    })
}
