import { BrandMark } from "../components/BrandMark";
import { useEffect, useState } from "react";
import {
  GitBranch,
  FileText,
  AlertCircle,
  HelpCircle,
} from "lucide-react";
import { NEXQ_VERSION, NEXQ_BUILD_DATE, NEXQ_DEVELOPER } from "../lib/version";
import { open } from "@tauri-apps/plugin-shell";
import { arch } from "@tauri-apps/plugin-os";
import { useUpdaterStore } from "../stores/updaterStore";
import { getPlatformCapabilities } from "../lib/ipc";
import type { PlatformCapabilities } from "../lib/types";

const GITHUB_URL = "https://github.com/aliozkanozdurmus/KebabM";

function formatBuildDate(dateStr: string): string {
  try {
    const d = new Date(dateStr);
    return d.toLocaleDateString("en-US", {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  } catch {
    return dateStr;
  }
}

export function AboutSettings() {
  const updateStatus = useUpdaterStore(s => s.checkStatus);
  const architecture = (() => { try { return arch(); } catch { return "Unavailable"; } })();
  const [platform, setPlatform] = useState<PlatformCapabilities | null>(null);

  useEffect(() => {
    getPlatformCapabilities().then(setPlatform).catch(() => {});
  }, []);

  const osLabel =
    platform?.os === "windows"
      ? "Windows"
      : platform?.os === "macos"
        ? "macOS"
        : platform?.os === "linux"
          ? "Linux"
          : "This computer";

  return (
    <div className="space-y-6">
      {/* App Identity Card */}
      <div className="rounded-xl border border-border/30 bg-card/50 p-6">
        <div className="flex items-start gap-5">
          <BrandMark className="h-16 w-16" />
          <div>
            <h3 className="text-lg font-bold text-foreground">KebabM</h3>
            <p className="text-xs text-muted-foreground">
              v{NEXQ_VERSION}
            </p>
            <p className="mt-2 text-sm text-muted-foreground">
              Project knowledge and live meeting assistance
            </p>
            <div className="mt-3 flex flex-wrap items-center gap-2">
              <span className="inline-flex items-center rounded-full bg-secondary/50 px-3 py-1 text-meta font-medium text-muted-foreground">
                Tauri 2
              </span>
              <span className="inline-flex items-center rounded-full bg-secondary/50 px-3 py-1 text-meta font-medium text-muted-foreground">
                React 19
              </span>
              <span className="inline-flex items-center rounded-full bg-secondary/50 px-3 py-1 text-meta font-medium text-muted-foreground">
                {osLabel}
              </span>
              {platform && !platform.stealth && (
                <span className="inline-flex items-center rounded-full bg-secondary/50 px-3 py-1 text-meta font-medium text-muted-foreground">
                  Screen-share hiding is Windows-only
                </span>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Meta Grid (2x2) */}
      <div className="grid grid-cols-2 gap-3">
        <div className="rounded-xl border border-border/30 bg-card/50 p-4">
          <p className="text-meta text-muted-foreground/60">Build Date</p>
          <p className="mt-1 text-sm font-medium text-foreground">
            {formatBuildDate(NEXQ_BUILD_DATE)}
          </p>
        </div>
        <div className="rounded-xl border border-border/30 bg-card/50 p-4">
          <p className="text-meta text-muted-foreground/60">Developer</p>
          <p className="mt-1 text-sm font-medium text-foreground">
            {NEXQ_DEVELOPER}
          </p>
        </div>
        <div className="rounded-xl border border-border/30 bg-card/50 p-4">
          <p className="text-meta text-muted-foreground/60">Architecture</p>
          <p className="mt-1 text-sm font-medium text-foreground">{architecture}</p>
        </div>
        <div className="rounded-xl border border-border/30 bg-card/50 p-4">
          <p className="text-meta text-muted-foreground/60">Copyright</p>
          <p className="mt-1 text-sm font-medium text-foreground">{NEXQ_DEVELOPER}</p>
        </div>
      </div>

      {/* Quick Links Row */}
      <div className="grid grid-cols-4 gap-3">
        <button
          onClick={() => open(GITHUB_URL)}
          className="flex flex-col items-center gap-2 rounded-xl border border-border/30 bg-card/50 p-4 transition-colors hover:bg-secondary/30"
        >
          <GitBranch className="h-4 w-4 text-muted-foreground" />
          <span className="text-meta font-medium text-muted-foreground">
            GitHub
          </span>
        </button>
        <button
          onClick={() => open(`${GITHUB_URL}/blob/main/CHANGELOG.md`)}
          className="flex flex-col items-center gap-2 rounded-xl border border-border/30 bg-card/50 p-4 transition-colors hover:bg-secondary/30"
        >
          <FileText className="h-4 w-4 text-muted-foreground" />
          <span className="text-meta font-medium text-muted-foreground">
            Changelog
          </span>
        </button>
        <button
          onClick={() => open(`${GITHUB_URL}/issues/new/choose`)}
          className="flex flex-col items-center gap-2 rounded-xl border border-border/30 bg-card/50 p-4 transition-colors hover:bg-secondary/30"
        >
          <AlertCircle className="h-4 w-4 text-muted-foreground" />
          <span className="text-meta font-medium text-muted-foreground">
            Report Issue
          </span>
        </button>
        <button
          onClick={() => open(`${GITHUB_URL}/wiki`)}
          className="flex flex-col items-center gap-2 rounded-xl border border-border/30 bg-card/50 p-4 transition-colors hover:bg-secondary/30"
        >
          <HelpCircle className="h-4 w-4 text-muted-foreground" />
          <span className="text-meta font-medium text-muted-foreground">
            Documentation
          </span>
        </button>
      </div>

      <p className="text-xs text-muted-foreground" role="status">
        {updateStatus === "disabled" ? "Signed updates are not configured for this build." : updateStatus === "error" ? "The update service could not be reached." : updateStatus === "available" ? "A signed update is available." : updateStatus === "up-to-date" ? "No newer update is available on the configured channel." : "Update channel status is being checked."}
      </p>
      {/* Footer */}
      <div className="rounded-xl border border-border/30 bg-card/50 p-5">
        <p className="text-xs text-muted-foreground/60 leading-relaxed">
          KebabM is a desktop application. All processing can run locally
          with Ollama or LM Studio, or optionally connect to cloud AI providers.
        </p>
      </div>
    </div>
  );
}
