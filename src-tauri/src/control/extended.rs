//! MCP delegates to the same application services as Tauri windows.
use super::*;
use crate::commands::{audio_commands as audio, context_commands as context, intelligence_commands as intelligence, meeting_commands as meetings, project_commands as project};
use crate::intelligence::session;

fn parsed(raw: String) -> Result<Value, String> { serde_json::from_str(&raw).map_err(|_| "Invalid service response".into()) }
fn boolean(args: &Value, key: &str) -> Result<bool,String> { args[key].as_bool().ok_or_else(||format!("{key} must be a boolean")) }
fn session_value(app: &AppHandle) -> Result<Value,String> { serde_json::to_value(session::get_assist_session(app.clone())?).map_err(|e|e.to_string()) }

pub(super) async fn dispatch(app: &AppHandle, tool: &str, args: &Value) -> Result<Value,String> {
    let id = || required_string(args,"id");
    match tool {
        "list_local_stt_models" => parsed(crate::commands::model_commands::list_local_stt_engines(app.clone()).await?),
        "download_local_stt_model" | "cancel_model_download" | "delete_local_stt_model" => {
            let engine=required_string(args,"engine")?;let model=required_string(args,"model_id")?;
            match tool {
                "download_local_stt_model"=>crate::commands::model_commands::download_local_stt_model(app.clone(),engine,model).await?,
                "cancel_model_download"=>crate::commands::model_commands::cancel_model_download(app.clone(),engine,model).await?,
                _=>crate::commands::model_commands::delete_local_stt_model(app.clone(),engine,model).await?,
            }
            Ok(json!({"accepted":true}))
        },
        "get_translation_config" => translation_config(app),
        "configure_translation" => {
            let provider=required_string(args,"provider")?;let target=required_string(args,"target_lang")?;
            let source=optional_string(args,"source_lang").unwrap_or_else(||"auto".into());
            crate::commands::translation_commands::set_translation_provider(app.clone(),provider.clone(),optional_string(args,"region")).await?;
            crate::commands::translation_commands::set_translation_languages(app.clone(),target.clone(),if source=="auto"{None}else{Some(source.clone())}).await?;
            let store=app.store("translation-config.json").map_err(|e|e.to_string())?;
            store.set("provider",json!(provider));store.set("targetLang",json!(target));store.set("sourceLang",json!(source));store.save().map_err(|e|e.to_string())?;
            translation_config(app)
        },
        "translate_text" => serde_json::to_value(crate::commands::translation_commands::translate_text(app.clone(),required_string(args,"text")?,required_string(args,"target_lang")?,optional_string(args,"source_lang")).await?).map_err(|e|e.to_string()),
        "test_translation" => serde_json::to_value(crate::commands::translation_commands::test_translation_connection(app.clone(),String::new()).await?).map_err(|e|e.to_string()),
        "meeting_translations" => serde_json::to_value(crate::commands::translation_commands::get_all_meeting_translations(app.clone(),id()?).await?).map_err(|e|e.to_string()),
        "knowledge_status" => parsed(project::project_knowledge_status(id()?,app.state()).await?),
        "search_knowledge" => parsed(project::search_project_knowledge(id()?,required_string(args,"question")?,app.state()).await?),
        "read_evidence" => Ok(json!({"id":id()?,"text":project::read_project_evidence(id()?,app.state()).await?})),
        "get_embedding_config" => serde_json::to_value(project::project_embedding_config(id()?,None,app.state()).await?).map_err(|e|e.to_string()),
        "set_embedding_config" => {
            let config=serde_json::from_value(args["config"].clone()).map_err(|_|"Invalid embedding config")?;
            serde_json::to_value(project::project_embedding_config(id()?,Some(config),app.state()).await?).map_err(|e|e.to_string())
        },
        "embed_project" => Ok(json!({"embeddedChunks":project::embed_project(id()?,app.clone()).await?})),
        "project_memory" => project::project_memory(id()?,app.state()).await,
        "save_open_question" => {project::save_open_question(id()?,required_string(args,"question")?,optional_string(args,"meeting_id"),app.state()).await?;Ok(json!({"saved":true}))},
        "review_decision" | "resolve_question" | "recheck_questions" => {
            let kind=match tool {"review_decision"=>"decision","resolve_question"=>"question",_=>"recheck"};
            project::update_project_memory(id()?,optional_string(args,"item_id"),kind.into(),None,None,optional_string(args,"status"),app.state()).await?;
            project::project_memory(id()?,app.state()).await
        },
        "project_preparation" => project::project_preparation(id()?,required_string(args,"kind")?,args["regenerate"].as_bool().unwrap_or(false),app.clone()).await,
        "draft_meeting_decisions" => project::draft_meeting_decisions(id()?,app.clone()).await,
        "list_documents" => parsed(context::list_context_resources(app.state()).await?),
        "load_document" => parsed(context::load_context_file(required_string(args,"path")?,app.state()).await?),
        "project_documents" => Ok(json!(project::project_documents(id()?,app.state()).await?)),
        "link_document" => {project::link_project_document(id()?,required_string(args,"resource_id")?,boolean(args,"linked")?,app.state()).await?;Ok(json!({"updated":true}))},
        "get_session" => session_value(app),
        "ask_question" => {
            let session_id=session::current_id(app).ok_or("Start a meeting before asking a question")?;
            let request_id=Uuid::new_v4().to_string();
            intelligence::generate_assist(match optional_string(args,"mode").as_deref().unwrap_or("assist") {"assist"=>"Assist","say"=>"Say","short"=>"Shorten","followup"=>"FollowUp","recap"=>"Recap",_=>return Err("Invalid answer mode".into())}.into(),Some(required_string(args,"question")?),None,Some(request_id.clone()),Some(session_id),None,None,app.clone(),app.state()).await?;
            Ok(json!({"requestId":request_id,"session":session_value(app)?}))
        },
        "answer_question" => {session::answer_session_question(id()?,app.clone()).await?;session_value(app)},
        "cancel_assist" => {intelligence::cancel_generation(app.state()).await?;session_value(app)},
        "set_reply_language" => {
            let language=required_string(args,"language")?;
            intelligence::set_ai_reply_language(language.clone(),app.state())?;
            let store=app.store(STORE_FILE).map_err(|e|e.to_string())?;store.set("aiReplyLanguage",json!(language));store.save().map_err(|e|e.to_string())?;
            Ok(json!({"language":language}))
        },
        "test_stt" => Ok(json!({"connected":crate::commands::stt_commands::test_stt_connection(app.clone(),required_string(args,"provider")?).await?})),
        "start_audio_test" => {audio::start_audio_test(app.clone(),required_string(args,"device_id")?,boolean(args,"is_input")?).await?;Ok(json!({"started":true}))},
        "stop_audio_test" => Ok(json!({"audioDetected":audio::stop_audio_test(app.clone()).await?})),
        "audio_status" => {
            let capturing=app.state::<AppState>().audio.lock().map_err(|_|"Audio state unavailable")?.as_ref().is_some_and(|m|m.is_capturing());
            Ok(json!({"capturing":capturing,"levels":parsed(audio::get_audio_level(app.clone()).await?)?,"mute":parsed(audio::get_mute_status(app.clone()).await?)?}))
        },
        "start_capture" => {
            if session::current_id(app).is_none(){return Err("Start a meeting before capture".into());}
            for (party, role) in [("you", "You"), ("them", "Them")] {
                if args[party]["role"].as_str() != Some(role) {return Err(format!("{} must use role {}", party, role));}
                if args[party]["stt_provider"].as_str() == Some("web_speech") {return Err("Browser speech recognition requires the frontend. Select a native, local or cloud speech provider for MCP capture.".into());}
            }
            audio::start_capture_per_party(app.clone(),args["you"].to_string(),args["them"].to_string(),required_string(args,"language")?).await?;Ok(json!({"started":true}))
        },
        "stop_capture" => {audio::stop_capture(app.clone()).await?;Ok(json!({"stopped":true}))},
        "set_source_muted" => {
            let source=match required_string(args,"source")?.as_str(){"mic"=>"you","system"=>"them",_=>return Err("Expected mic or system".into())};
            audio::set_source_muted(app.clone(),source.into(),boolean(args,"muted")?).await?;parsed(audio::get_mute_status(app.clone()).await?)
        },
        "start_meeting" => {
            if session::current_id(app).is_some(){return Err("A meeting is already active; end it first".into());}
            let value=parsed(meetings::start_meeting(optional_string(args,"title"),app.state()).await?)?;
            session::publish(app);Ok(value)
        },
        "end_meeting" => {
            let id=session::current_id(app).ok_or("No active meeting")?;
            let capturing=app.state::<AppState>().audio.lock().map_err(|_|"Audio state unavailable")?.as_ref().is_some_and(|m|m.is_capturing());
            if capturing{audio::stop_capture(app.clone()).await?;}
            meetings::end_meeting(id.clone(),app.state(),app.clone()).await?;Ok(json!({"id":id,"ended":true}))
        },
        _=>Err(format!("Unknown tool: {tool}")),
    }
}

fn translation_config(app: &AppHandle) -> Result<Value,String> {
    let store=app.store("translation-config.json").map_err(|e|e.to_string())?;
    let state=app.state::<AppState>();
    let router=state.translation.as_ref().ok_or("Translation not initialized")?.lock().map_err(|_|"Translation state unavailable")?;
    Ok(json!({"saved":{"provider":store.get("provider"),"targetLang":store.get("targetLang"),"sourceLang":store.get("sourceLang")},"active":{"provider":router.active_provider_name(),"targetLang":router.default_target_lang(),"sourceLang":router.default_source_lang()}}))
}
