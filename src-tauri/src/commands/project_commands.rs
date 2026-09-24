use tauri::{command, AppHandle, Emitter, Listener, Manager, State};

use crate::llm::provider::GenerationParams;
use crate::llm::provider::LLMMessage;
use crate::projects;
use crate::state::AppState;

#[command]
pub async fn list_projects(state: State<'_, AppState>) -> Result<String, String> {
    let db = lock_db(&state)?;
    let projects = projects::list(db.connection())?;
    serde_json::to_string(&projects).map_err(|e| e.to_string())
}

#[command]
pub async fn create_project(name: String, root_path: String, state: State<'_, AppState>) -> Result<String, String> {
    let name = name.trim().to_string();
    let root_path = root_path.trim().to_string();
    if name.is_empty() || root_path.is_empty() {
        return Err("Project name and folder are required".to_string());
    }
    let db = lock_db(&state)?;
    let project = projects::create(db.connection(), &name, &root_path)?;
    serde_json::to_string(&project).map_err(|e| e.to_string())
}

#[command]
pub async fn delete_project(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = lock_db(&state)?;
    projects::delete(db.connection(), &id)
}

#[command]
pub async fn set_active_project(id: Option<String>, state: State<'_, AppState>) -> Result<(), String> {
    let db = lock_db(&state)?;
    projects::set_active(db.connection(), id.as_deref())
}

const FINAL_PROMPT: &str = "\
You are a code agent who was asked: what is this project, and what must a person know to sit in a meeting about it? \
The scout pack below is how you would open the repo: folder map, product files, and a short surface of the other files. \
You did not read every line, and you must not pretend you did. \
Write in Turkish. Keep paths and code names as they appear. Do not invent files or behavior. \
Reply with exactly these sections, each starting with its marker line and nothing else on that line: \
<<<FILE overview.md>>> \
<<<FILE architecture.md>>> \
<<<FILE flows.md>>> \
<<<FILE domain.md>>> \
<<<FILE where-to-look.md>>> \
<<<FILE risks.md>>> \
<<<FILE meeting-guide.md>>> \
overview.md: what it is, who it is for, and the promise, in a few short paragraphs. \
architecture.md: the real structure and how the main parts talk. \
flows.md: the flows a meeting will ask about. \
domain.md: the words and rules the team uses. \
where-to-look.md: which path to open for each kind of question. \
risks.md: sharp edges that are actually visible in the scout pack. \
meeting-guide.md: how to answer, what to cite, and what the scout pack does not show. \
When a section describes structure, include one mermaid diagram. Keep every section tight.";

#[command]
pub fn set_meeting_feed(text: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut feed = state.meeting_feed.write().map_err(|e| e.to_string())?;
    *feed = text;
    Ok(())
}

#[command]
pub async fn scan_project(app: AppHandle, id: String) -> Result<String, String> {
    let state = app.state::<AppState>();
    emit_progress(&app, "read", "Mapping the project");

    let (root, dossier) = {
        let db = lock_db(&state)?;
        let project = projects::get(db.connection(), &id)?;
        let dossier = projects::prepare_index(db.connection(), &id)?;
        (std::path::PathBuf::from(project.root_path), dossier)
    };
    if dossier.trim().is_empty() {
        return Err("No readable source or docs were found in that folder.".to_string());
    }

    emit_progress(&app, "synthesize", "Writing what the project is");
    let written = ask_model(&app, &state, &format!("{FINAL_PROMPT}\n\n{dossier}")).await?;
    let sections = split_knowledge_files(&written);
    let overview = sections
        .iter()
        .find(|(name, _)| name == "overview.md")
        .map(|(_, body)| body.clone())
        .unwrap_or_else(|| written.clone());
    if sections.is_empty() {
        projects::write_knowledge(&root, "knowledge-base.md", &written)?;
    } else {
        for (name, body) in &sections {
            projects::write_knowledge(&root, name, body)?;
        }
    }

    let db = lock_db(&state)?;
    let project = projects::finish_scan(db.connection(), &id, &overview)?;
    emit_progress(&app, "done", "zaiqo-meet is ready");
    serde_json::to_string(&project).map_err(|e| e.to_string())
}

fn split_knowledge_files(text: &str) -> Vec<(String, String)> {
    let mut sections = Vec::new();
    let mut current_name: Option<String> = None;
    let mut body = String::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_prefix("<<<FILE ").and_then(|rest| rest.strip_suffix(">>>")) {
            if let Some(name) = current_name.take() {
                let text = body.trim().to_string();
                if !text.is_empty() {
                    sections.push((name, text));
                }
                body.clear();
            }
            current_name = Some(name.trim().to_string());
            continue;
        }
        if current_name.is_some() {
            body.push_str(line);
            body.push('\n');
        }
    }
    if let Some(name) = current_name {
        let text = body.trim().to_string();
        if !text.is_empty() {
            sections.push((name, text));
        }
    }
    sections
}

fn emit_progress(app: &AppHandle, stage: &str, detail: &str) {
    let _ = app.emit("project-scan", serde_json::json!({ "stage": stage, "detail": detail }));
}

async fn ask_model(app: &AppHandle, state: &AppState, prompt: &str) -> Result<String, String> {
    let (provider, model) = {
        let llm = state.llm.as_ref().ok_or_else(|| {
            "Choose a model in LLM settings before scanning a project.".to_string()
        })?;
        let router = llm.lock().map_err(|e| e.to_string())?;
        let provider = router.get_provider().map_err(|_| {
            "Choose a model in LLM settings before scanning a project.".to_string()
        })?;
        let model = router.active_model().to_string();
        if model.is_empty() {
            return Err("Choose a model in LLM settings before scanning a project.".to_string());
        }
        (provider, model)
    };

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let listener = app.listen("llm_stream_token", move |event| {
        if let Ok(payload) = serde_json::from_str::<crate::llm::provider::StreamTokenPayload>(event.payload()) {
            let _ = tx.send(payload.token);
        }
    });

    let messages = vec![
        LLMMessage {
            role: "system".to_string(),
            content: "You are a code agent writing a meeting knowledge base in Turkish. Never invent files, symbols, or behavior that are not in the material. Keep code names and paths unchanged.".to_string(),
        },
        LLMMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        },
    ];
    let params = GenerationParams {
        temperature: Some(0.2),
        ..GenerationParams::default()
    };
    let result = {
        let guard = provider.lock().await;
        guard.stream_completion(messages, &model, params, app.clone()).await
    };
    app.unlisten(listener);
    result.map_err(|e| format!("The model could not read this project: {e}"))?;

    let mut text = String::new();
    while let Ok(token) = rx.try_recv() {
        text.push_str(&token);
    }
    let text = text.trim().to_string();
    if text.is_empty() {
        Err("The model returned an empty knowledge note. Check the model and scan again.".to_string())
    } else {
        Ok(text)
    }
}

fn lock_db(state: &AppState) -> Result<std::sync::MutexGuard<'_, crate::db::DatabaseManager>, String> {
    let db = state.database.as_ref().ok_or_else(|| "Database not initialized".to_string())?;
    db.lock().map_err(|e| e.to_string())
}
