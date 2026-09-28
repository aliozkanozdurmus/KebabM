const str = (description) => ({ type: "string", description });
const obj = (properties, required = []) => ({
  type: "object",
  properties,
  required,
  additionalProperties: false,
});

export const legacyTools = [
  { name: "status", description: "Zaiqo snapshot: language, saved settings, active project, open meeting, which secrets exist (names only), and platform. The desktop app must be open.", inputSchema: obj({}) },
  { name: "platform", description: "What this operating system can do: mic, system audio, stealth, native speech recognition.", inputSchema: obj({}) },
  { name: "get_settings", description: "Read the saved app settings. Secrets are not included.", inputSchema: obj({}) },
  {
    name: "set_settings",
    description: "Change saved settings and apply them in the running app. Pass a values object. Use firstRunCompleted true to leave the welcome wizard. Allowed keys include appearance (ibm, liquid-glass, apple, linear, notion, material, github, terminal), theme, sttProvider, sttLanguage, llmProvider, llmModel, meetingAudioConfig, recordingEnabled, deepgramConfig, groqConfig, pauseThresholdMs, autoTrigger, autoSummary, firstRunCompleted, overlayOpacity, and the tray toggles. API keys are not settings; use store_secret.",
    inputSchema: obj({
      values: { type: "object", description: "Setting key to JSON value." },
    }),
  },
  { name: "list_llm_providers", description: "List model providers Zaiqo can use, including xai, chatgpt, azure, openai, and the OpenAI-compatible clouds.", inputSchema: obj({}) },
  {
    name: "configure_llm",
    description: "Select the model provider and model, and optionally store an API key. The key is saved in the OS credential store and is not returned. For Azure, base_url is the resource endpoint. For ChatGPT subscription use sign_in_chatgpt instead of an API key. provider examples: xai, openai, anthropic, gemini, azure, ollama, openrouter, chatgpt.",
    inputSchema: obj(
      {
        provider: str("Provider id, for example xai or azure."),
        model: str("Model or Azure deployment name."),
        api_key: str("API key. Omitted from the response."),
        base_url: str("Azure resource URL, or a custom base URL."),
      },
      ["provider"]
    ),
  },
  {
    name: "list_models",
    description: "List models for the active provider. Pass provider to select one first.",
    inputSchema: obj({ provider: str("Provider id."), model: str("Model to save while selecting the provider."), api_key: str("API key, if it is not stored yet.") }),
  },
  {
    name: "test_llm",
    description: "Check that the selected model provider answers. Pass provider to select and save it first.",
    inputSchema: obj({ provider: str("Provider id."), model: str("Model id."), api_key: str("API key, if it is not stored yet."), base_url: str("Azure or custom base URL.") }),
  },
  { name: "sign_in_chatgpt", description: "Start the official ChatGPT browser sign-in. This call waits until the login finishes. Zaiqo must be open.", inputSchema: obj({}) },
  {
    name: "store_secret",
    description: "Save a secret in the OS credential store. The value is not returned. provider is a short name such as xai, deepgram, azure, azure_endpoint, or azure_speech_region.",
    inputSchema: obj({ provider: str("Secret name."), key: str("Secret value.") }, ["provider", "key"]),
  },
  { name: "delete_secret", description: "Delete one saved secret by its short name.", inputSchema: obj({ provider: str("Secret name.") }, ["provider"]) },
  { name: "list_configured_secrets", description: "List secret names that are saved. Values are not returned.", inputSchema: obj({}) },
  { name: "list_stt_providers", description: "List speech recognition providers.", inputSchema: obj({}) },
  {
    name: "configure_stt",
    description: "Set the recognition language and providers. language is BCP-47, for example fr-FR or tr-TR. you_provider and them_provider are the per-side engines, such as web_speech, windows_native, deepgram, groq_whisper, whisper_api, azure_speech, parakeet_tdt. Device ids come from list_audio_devices.",
    inputSchema: obj({
      language: str("BCP-47 language, for example fr-FR."),
      provider: str("Top-level speech provider id."),
      you_provider: str("Microphone side provider."),
      them_provider: str("System-audio side provider."),
      you_device_id: str("Microphone device id."),
      them_device_id: str("System output device id."),
      pause_ms: { type: "integer", description: "Silence before a phrase is closed, in milliseconds." },
      deepgram: { type: "object", description: "Deepgram options such as model and diarize." },
      groq: { type: "object", description: "Groq Whisper options." },
    }),
  },
  { name: "list_audio_devices", description: "List microphone and output devices.", inputSchema: obj({}) },
  { name: "list_projects", description: "List project knowledge bases.", inputSchema: obj({}) },
  { name: "get_project", description: "Read one project, its knowledge brief, and module cards. Omit id to read the active project.", inputSchema: obj({ id: str("Project id.") }) },
  { name: "create_project", description: "Create a project from a local folder. This does not scan until scan_project.", inputSchema: obj({ name: str("Display name."), root_path: str("Absolute folder path.") }, ["name", "root_path"]) },
  { name: "set_active_project", description: "Choose which project the next meeting uses. Pass null or an empty id to clear it.", inputSchema: obj({ id: str("Project id, or empty to clear.") }) },
  { name: "delete_project", description: "Delete a project knowledge base. Meeting records stay.", inputSchema: obj({ id: str("Project id.") }, ["id"]) },
  { name: "scan_project", description: "Read the project with the selected model and write the knowledge base. Zaiqo must be open and a model must already be selected. This can take several minutes.", inputSchema: obj({ id: str("Project id.") }, ["id"]) },
  { name: "list_meetings", description: "List recent meetings, including project and source.", inputSchema: obj({ limit: { type: "integer" }, offset: { type: "integer" } }) },
  { name: "get_meeting", description: "Read one meeting, including its transcript.", inputSchema: obj({ id: str("Meeting id.") }, ["id"]) },
  { name: "search_meetings", description: "Search meeting titles and transcripts.", inputSchema: obj({ query: str("Search text.") }, ["query"]) },
  {
    name: "import_transcript",
    description: "Save a pasted transcript on a project. Lines can look like 'Ali: hello' or '00:12 Sam: hello'.",
    inputSchema: obj({ project_id: str("Project id."), title: str("Meeting title."), transcript: str("Transcript text.") }, ["project_id", "title", "transcript"]),
  },
  {
    name: "start_meeting",
    description: "Ask the open Zaiqo window to start a live meeting with the active project. audio_mode is online or in_person. scenario is team_meeting, lecture, interview, webinar, oral_exam, or custom.",
    inputSchema: obj({ title: str("Optional meeting title."), audio_mode: str("online or in_person."), scenario: str("Meeting scenario.") }),
  },
  { name: "end_meeting", description: "Ask the open Zaiqo window to end the live meeting and save it.", inputSchema: obj({}) },
  { name: "rename_meeting", description: "Rename a stored meeting.", inputSchema: obj({ id: str("Meeting id."), title: str("New title.") }, ["id", "title"]) },
  { name: "delete_meeting", description: "Delete a stored meeting.", inputSchema: obj({ id: str("Meeting id.") }, ["id"]) },
  { name: "set_custom_instructions", description: "Save the standing instructions the meeting assistant should follow. Pass an empty string to clear them.", inputSchema: obj({ text: str("Instruction text.") }, ["text"]) },
];
