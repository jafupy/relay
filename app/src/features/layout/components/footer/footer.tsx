import {
  AlertCircle,
  Download,
  Puzzle,
  Settings2,
  Sparkles,
  Terminal as TerminalIcon,
  X,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useExtensionStore } from "@/extensions/registry/extension-store";
import { useAIChatStore } from "@/features/ai/store/store";
import { useDiagnosticsStore } from "@/features/diagnostics/stores/diagnostics-store";
import { getGitStatus } from "@/features/git/api/git-status-api";
import GitBranchManager from "@/features/git/components/git-branch-manager";
import { useGitStore } from "@/features/git/stores/git-store";
import { useUpdater } from "@/features/settings/hooks/use-updater";
import { useSettingsStore } from "@/features/settings/store";
import { useDesktopSignIn } from "@/features/window/hooks/use-desktop-sign-in";
import { useAuthStore } from "@/features/window/stores/auth-store";
import { useUIState } from "@/features/window/stores/ui-state-store";
import Badge from "@/ui/badge";
import { Button } from "@/ui/button";
import { Dropdown } from "@/ui/dropdown";
import { cn } from "@/utils/cn";
import { useFileSystemStore } from "../../../file-system/controllers/store";

const AiUsageStatusIndicator = () => {
  const [isOpen, setIsOpen] = useState(false);
  const buttonRef = useRef<HTMLButtonElement>(null);

  const isAuthenticated = useAuthStore((state) => state.isAuthenticated);
  const subscription = useAuthStore((state) => state.subscription);
  const checkAllProviderApiKeys = useAIChatStore((state) => state.checkAllProviderApiKeys);
  const hasOpenRouterKey = useAIChatStore(
    (state) => state.providerApiKeys.get("openrouter") || false,
  );
  const { signIn, isSigningIn } = useDesktopSignIn({
    onSuccess: () => setIsOpen(false),
  });
  const uiState = useUIState();
  const hasBlockingModalOpen = useUIState(
    (state) =>
      state.isQuickOpenVisible ||
      state.isCommandPaletteVisible ||
      state.isGlobalSearchVisible ||
      state.isSettingsDialogVisible ||
      state.isThemeSelectorVisible ||
      state.isIconThemeSelectorVisible ||
      state.isProjectPickerVisible ||
      state.isDatabaseConnectionVisible,
  );

  const enterprisePolicy = subscription?.enterprise?.policy;
  const managedPolicy = enterprisePolicy?.managedMode ? enterprisePolicy : null;
  const aiAllowedByPolicy = managedPolicy ? managedPolicy.aiCompletionEnabled : true;
  const byokAllowedByPolicy = managedPolicy ? managedPolicy.allowByok : true;
  const planLabel = isAuthenticated ? "Local" : "Guest";
  const usesByok = isAuthenticated;

  const modeLabel = (() => {
    if (!isAuthenticated) return "Guest";
    if (!aiAllowedByPolicy) return "Blocked";
    if (!byokAllowedByPolicy) return "Blocked";
    return hasOpenRouterKey ? "BYOK" : "Key required";
  })();

  const indicatorLabel = !isAuthenticated ? "Guest" : planLabel;

  const modeToneClass = (() => {
    if (!isAuthenticated || !aiAllowedByPolicy) return "text-red-400";
    if (usesByok && !hasOpenRouterKey) return "text-yellow-400";
    if (usesByok) return "text-blue-400";
    return "text-emerald-400";
  })();

  const refreshAll = async () => {
    await checkAllProviderApiKeys();
  };

  useEffect(() => {
    void refreshAll();
  }, [checkAllProviderApiKeys]);

  useEffect(() => {
    if (!isOpen) return;
    void refreshAll();
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen || !hasBlockingModalOpen) return;
    setIsOpen(false);
  }, [hasBlockingModalOpen, isOpen]);

  const handleSignIn = async () => {
    await signIn();
  };

  return (
    <div className="relative">
      <Button
        ref={buttonRef}
        type="button"
        onClick={() => {
          const nextOpen = !isOpen;
          setIsOpen(nextOpen);
          if (nextOpen) {
            void refreshAll();
          }
        }}
        variant="secondary"
        size="xs"
        className={cn(
          "rounded-md bg-primary-bg/40 px-2 text-text-lighter",
          "ui-font ui-text-sm gap-1 font-medium",
          modeToneClass,
          isOpen && "border-border/60 bg-hover/80",
        )}
        style={{ minHeight: 0, minWidth: 0 }}
        tooltip={`${planLabel} • ${modeLabel}`}
      >
        <span className="ui-font ui-text-sm">{indicatorLabel}</span>
      </Button>
      <Dropdown
        isOpen={isOpen}
        anchorRef={buttonRef}
        anchorSide="top"
        anchorAlign="end"
        onClose={() => setIsOpen(false)}
        className="w-[320px] overflow-hidden rounded-xl p-0"
      >
        <div className="flex items-center justify-between border-border/70 border-b bg-secondary-bg/55 px-3 py-2.5">
          <div className="flex items-center gap-2">
            <span className="ui-font ui-text-md font-medium text-text">AI</span>
            <Badge variant="accent" shape="pill" size="compact" className="text-emerald-300">
              Local
            </Badge>
          </div>
          <div className="flex items-center gap-1.5">
            <Button
              onClick={() => {
                setIsOpen(false);
                uiState.openSettingsDialog("ai");
              }}
              variant="secondary"
              size="icon-sm"
              className="px-0 text-text-lighter"
              tooltip="AI Settings"
              aria-label="Open AI settings"
            >
              <Settings2 />
            </Button>
            <Button
              onClick={() => setIsOpen(false)}
              variant="secondary"
              size="icon-sm"
              className="px-0 text-text-lighter"
              aria-label="Close AI status dropdown"
            >
              <X />
            </Button>
          </div>
        </div>
        {!isAuthenticated ? (
          <div className="p-2.5">
            <Button
              onClick={() => void handleSignIn()}
              disabled={isSigningIn}
              variant="primary"
              size="sm"
              className="mt-2 w-full justify-center rounded-lg text-white hover:opacity-90"
            >
              {isSigningIn ? "Signing in..." : "Sign in"}
            </Button>
          </div>
        ) : null}
      </Dropdown>
    </div>
  );
};

const Footer = () => {
  const isAuthenticated = useAuthStore((state) => state.isAuthenticated);
  const settings = useSettingsStore((state) => state.settings);
  const uiState = useUIState();
  const { rootFolderPath } = useFileSystemStore();
  const workspaceGitStatus = useGitStore((state) => state.workspaceGitStatus);
  const { actions } = useGitStore();
  const { available, downloading, installing, updateInfo, downloadAndInstall } = useUpdater(false);

  const extensionUpdatesCount = useExtensionStore.use.extensionsWithUpdates().size;
  const diagnosticsByFile = useDiagnosticsStore.use.diagnosticsByFile();
  const diagnosticsCount = Array.from(diagnosticsByFile.values()).reduce(
    (total, diagnostics) => total + diagnostics.length,
    0,
  );

  return (
    <div className="relative z-20 flex min-h-9 shrink-0 items-center justify-between border-t border-border/40 bg-secondary-bg/80 px-2.5 py-1 backdrop-blur-sm">
      <div className="ui-font ui-text-sm flex items-center gap-1 text-text-lighter">
        {/* Git branch manager */}
        {rootFolderPath && workspaceGitStatus?.branch && (
          <GitBranchManager
            currentBranch={workspaceGitStatus.branch}
            repoPath={rootFolderPath}
            paletteTarget
            placement="up"
            onBranchChange={async () => {
              const status = await getGitStatus(rootFolderPath);
              actions.setWorkspaceGitStatus(status, rootFolderPath);
            }}
            compact={true}
          />
        )}

        {/* Terminal indicator */}
        {settings.coreFeatures.terminal && (
          <Button
            onClick={() => {
              uiState.setBottomPaneActiveTab("terminal");
              const showingTerminal =
                !uiState.isBottomPaneVisible || uiState.bottomPaneActiveTab !== "terminal";
              uiState.setIsBottomPaneVisible(showingTerminal);

              if (showingTerminal) {
                setTimeout(() => {
                  uiState.requestTerminalFocus();
                }, 100);
              }
            }}
            variant="secondary"
            size="icon-sm"
            className="rounded-md bg-primary-bg/40 text-text-lighter"
            data-active={uiState.isBottomPaneVisible && uiState.bottomPaneActiveTab === "terminal"}
            style={{ minHeight: 0, minWidth: 0 }}
            tooltip="Toggle Terminal"
            commandId="workbench.toggleTerminal"
          >
            <TerminalIcon />
          </Button>
        )}

        {/* Diagnostics indicator - clickable */}
        {settings.coreFeatures.diagnostics && (
          <Button
            onClick={() => {
              uiState.setBottomPaneActiveTab("diagnostics");
              const showingDiagnostics =
                !uiState.isBottomPaneVisible || uiState.bottomPaneActiveTab !== "diagnostics";
              uiState.setIsBottomPaneVisible(showingDiagnostics);
            }}
            variant="secondary"
            size="xs"
            className={cn(
              "rounded-md bg-primary-bg/40 px-2 text-text-lighter",
              !(uiState.isBottomPaneVisible && uiState.bottomPaneActiveTab === "diagnostics") &&
                diagnosticsCount > 0 &&
                "text-warning",
            )}
            data-active={
              uiState.isBottomPaneVisible && uiState.bottomPaneActiveTab === "diagnostics"
            }
            style={{ minHeight: 0, minWidth: 0 }}
            tooltip={
              diagnosticsCount > 0
                ? `${diagnosticsCount} diagnostic${diagnosticsCount === 1 ? "" : "s"}`
                : "Toggle Diagnostics Panel"
            }
            commandId="workbench.toggleDiagnostics"
          >
            <AlertCircle />
            {diagnosticsCount > 0 && <span className="ui-text-sm ml-0.5">{diagnosticsCount}</span>}
          </Button>
        )}
        {/* Extension updates indicator */}
        {extensionUpdatesCount > 0 && (
          <Button
            onClick={() => uiState.openSettingsDialog("extensions")}
            variant="secondary"
            size="xs"
            className="rounded-md bg-primary-bg/40 px-2 text-blue-400"
            style={{ minHeight: 0, minWidth: 0 }}
            tooltip={`${extensionUpdatesCount} extension update${extensionUpdatesCount === 1 ? "" : "s"} available`}
          >
            <Puzzle />
            <span className="ui-text-sm ml-0.5">{extensionUpdatesCount}</span>
          </Button>
        )}
        {/* Update indicator */}
        {available && (
          <Button
            onClick={downloadAndInstall}
            disabled={downloading || installing}
            variant="secondary"
            size="icon-sm"
            className={cn(
              "rounded-md bg-primary-bg/40 text-text-lighter",
              downloading || installing
                ? "cursor-not-allowed opacity-60"
                : "text-blue-400 hover:text-blue-300",
            )}
            style={{ minHeight: 0, minWidth: 0 }}
            tooltip={
              downloading
                ? "Downloading update..."
                : installing
                  ? "Installing update..."
                  : `Update available: ${updateInfo?.version}`
            }
          >
            <Download className={downloading || installing ? "animate-pulse" : ""} />
          </Button>
        )}
      </div>

      <div className="ui-font ui-text-sm flex items-center gap-1 text-text-lighter">
        {isAuthenticated && <AiUsageStatusIndicator />}

        {/* AI Chat button */}
        <Button
          onClick={() => {
            useSettingsStore.getState().toggleAIChatVisible();
          }}
          variant="secondary"
          size="icon-sm"
          className="rounded-md bg-primary-bg/40 text-text-lighter"
          data-active={settings.isAIChatVisible}
          style={{ minHeight: 0, minWidth: 0 }}
          tooltip="Toggle AI Chat"
          commandId="workbench.toggleAIChat"
        >
          <Sparkles />
        </Button>
      </div>
    </div>
  );
};

export default Footer;
