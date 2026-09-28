// Hook for managing the full update lifecycle: startup check, periodic checks,
// skip version persistence, download, and restart.

import { useEffect, useRef, useCallback } from "react";
import { useUpdaterStore } from "../stores/updaterStore";
import {
  checkForUpdate,
  updaterEnabled,
  downloadAndInstallUpdate,
  restartForUpdate,
} from "../lib/ipc";
import {
  onUpdateDownloadProgress,
  onUpdateReady,
} from "../lib/events";
import { load } from "@tauri-apps/plugin-store";
import type { UnlistenFn } from "@tauri-apps/api/event";

const STORE_NAME = "nexq-settings.json";
const SKIPPED_VERSION_KEY = "skipped_version";

export function useUpdater() {
  const store = useUpdaterStore();
  const downloadedRef = useRef(0);
  const mountedRef = useRef(true);

  // -- performCheck ----------------------------------------------------------

  const performCheck = useCallback(async () => {
    const state = useUpdaterStore.getState();
    if (state.checkStatus === "checking" || state.downloadStatus === "downloading") return;
    state.setCheckStatus("checking");
    try {
      if (!(await updaterEnabled())) {
        if (mountedRef.current) state.setCheckStatus("disabled");
        return;
      }
      const update = await checkForUpdate();
      if (!mountedRef.current) return;
      state.setAvailableUpdate(update?.version === useUpdaterStore.getState().skippedVersion ? null : update);
    } catch {
      if (mountedRef.current) state.setCheckError("Update check failed. Check the connection and try again.");
    }
  }, []);

  // -- startDownload ---------------------------------------------------------

  const startDownload = useCallback(async () => {
    const { setDownloadStatus, setDownloadProgress } =
      useUpdaterStore.getState();

    setDownloadStatus("downloading");
    setDownloadProgress(0, null);
    downloadedRef.current = 0;

    try {
      await downloadAndInstallUpdate();
    } catch (err: unknown) {
      if (!mountedRef.current) return;
      const msg =
        typeof err === "string"
          ? err
          : err instanceof Error
            ? err.message
            : "Unknown error";
      console.error("[useUpdater] Download failed:", msg);
      useUpdaterStore.getState().setDownloadStatus("error");
    }
  }, []);

  // -- restart ---------------------------------------------------------------

  const restart = useCallback(async () => {
    try {
      await restartForUpdate();
    } catch (err) {
      console.error("[useUpdater] Restart failed:", err);
    }
  }, []);

  // -- skipVersion -----------------------------------------------------------

  const skipVersion = useCallback(async (version: string) => {
    const { setSkippedVersion, setAvailableUpdate, setCheckStatus } =
      useUpdaterStore.getState();

    setSkippedVersion(version);
    setAvailableUpdate(null);
    setCheckStatus("up-to-date");

    try {
      const tauriStore = await load(STORE_NAME, { autoSave: true, defaults: {} });
      await tauriStore.set(SKIPPED_VERSION_KEY, version);
    } catch (err) {
      console.warn("[useUpdater] Failed to persist skipped version:", err);
    }
  }, []);

  // -- mount: load persisted skip, set up events, startup check, interval ----

  useEffect(() => {
    mountedRef.current = true;

    let unlistenProgress: UnlistenFn | null = null;
    let unlistenReady: UnlistenFn | null = null;

    // Load persisted skipped version
    load(STORE_NAME, { autoSave: true, defaults: {} })
      .then(async (tauriStore) => {
        const skipped = await tauriStore.get<string>(SKIPPED_VERSION_KEY);
        if (skipped && mountedRef.current) {
          useUpdaterStore.getState().setSkippedVersion(skipped);
        }
      })
      .catch(() => {})
      .finally(() => { if (mountedRef.current) void performCheck(); });
    const interval = window.setInterval(() => { void performCheck(); }, 6 * 60 * 60 * 1000);

    // Set up event listeners
    onUpdateDownloadProgress((event) => {
      if (!mountedRef.current) return;
      downloadedRef.current += event.chunk_length;
      useUpdaterStore
        .getState()
        .setDownloadProgress(downloadedRef.current, event.content_length);
    }).then((fn) => {
      if (!mountedRef.current) fn(); else unlistenProgress = fn;
    });

    onUpdateReady(() => {
      if (!mountedRef.current) return;
      useUpdaterStore.getState().setDownloadStatus("ready");
    }).then((fn) => {
      if (!mountedRef.current) fn(); else unlistenReady = fn;
    });

    return () => {
      mountedRef.current = false;
      window.clearInterval(interval);
      unlistenProgress?.();
      unlistenReady?.();
    };
  }, [performCheck]);

  return {
    // Store state
    checkStatus: store.checkStatus,
    lastChecked: store.lastChecked,
    availableUpdate: store.availableUpdate,
    checkError: store.checkError,
    downloadStatus: store.downloadStatus,
    downloadedBytes: store.downloadedBytes,
    totalBytes: store.totalBytes,
    skippedVersion: store.skippedVersion,

    // Actions
    performCheck,
    startDownload,
    restart,
    skipVersion,
  };
}
