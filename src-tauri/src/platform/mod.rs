use serde::Serialize;

/// What this operating system can do today.
///
/// Mic capture is cpal on every desktop OS. System audio is WASAPI loopback
/// on Windows, a PulseAudio monitor (PipeWire compatible) on Linux, and
/// ScreenCaptureKit on macOS. Capture exclusion and SAPI stay Windows-only.
#[derive(Debug, Clone, Serialize)]
pub struct PlatformCapabilities {
    pub os: &'static str,
    pub mic_capture: bool,
    pub system_audio: bool,
    pub stealth: bool,
    pub credential_store: bool,
    pub native_stt: bool,
}

pub fn current() -> PlatformCapabilities {
    #[cfg(target_os = "windows")]
    {
        return PlatformCapabilities {
            os: "windows",
            mic_capture: true,
            system_audio: true,
            stealth: true,
            credential_store: true,
            native_stt: true,
        };
    }

    #[cfg(target_os = "macos")]
    {
        return PlatformCapabilities {
            os: "macos",
            mic_capture: true,
            system_audio: true,
            stealth: false,
            credential_store: true,
            native_stt: false,
        };
    }

    #[cfg(target_os = "linux")]
    {
        return PlatformCapabilities {
            os: "linux",
            mic_capture: true,
            system_audio: true,
            stealth: false,
            credential_store: true,
            native_stt: false,
        };
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        PlatformCapabilities {
            os: "unknown",
            mic_capture: false,
            system_audio: false,
            stealth: false,
            credential_store: false,
            native_stt: false,
        }
    }
}

#[tauri::command]
pub fn get_platform_capabilities() -> PlatformCapabilities {
    current()
}
