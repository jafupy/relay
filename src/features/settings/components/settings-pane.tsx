import { Search } from "lucide-react";
import { useEffect, useState } from "react";
import { useSettingsStore } from "@/features/settings/store";
import { useAuthStore } from "@/features/window/stores/auth-store";
import { type SettingsTab, useUIState } from "@/features/window/stores/ui-state-store";
import Input from "@/ui/input";
import { SettingsVerticalTabs } from "./settings-vertical-tabs";
import { AccountSettings } from "./tabs/account-settings";
import { AdvancedSettings } from "./tabs/advanced-settings";
import { AISettings } from "./tabs/ai-settings";
import { AppearanceSettings } from "./tabs/appearance-settings";
import { DatabaseSettings } from "./tabs/database-settings";
import { EditorSettings } from "./tabs/editor-settings";
import { EnterpriseSettings } from "./tabs/enterprise-settings";
import { ExtensionsSettings } from "./tabs/extensions-settings";
import { FeaturesSettings } from "./tabs/features-settings";
import { FileTreeSettings } from "./tabs/file-tree-settings";
import { GeneralSettings } from "./tabs/general-settings";
import { GitSettings } from "./tabs/git-settings";
import { KeyboardSettings } from "./tabs/keyboard-settings";
import { LanguageSettings } from "./tabs/language-settings";
import { TerminalSettings } from "./tabs/terminal-settings";

interface SettingsPaneProps {
  initialTab?: SettingsTab;
}

export function SettingsPane({ initialTab }: SettingsPaneProps) {
  const { settingsInitialTab, setSettingsInitialTab } = useUIState();
  const [activeTab, setActiveTab] = useState<SettingsTab>(
    initialTab ?? settingsInitialTab ?? "general",
  );
  const subscription = useAuthStore((state) => state.subscription);
  const hasEnterpriseAccess = Boolean(subscription?.enterprise?.has_access);

  const clearSearch = useSettingsStore((state) => state.clearSearch);
  const searchQuery = useSettingsStore((state) => state.search.query);
  const setSearchQuery = useSettingsStore((state) => state.setSearchQuery);

  const handleTabChange = (tab: SettingsTab) => {
    setActiveTab(tab);
    setSettingsInitialTab(tab);
  };

  // Sync with global settings tab when it changes externally
  useEffect(() => {
    if (initialTab) setActiveTab(initialTab);
  }, [initialTab]);

  useEffect(() => {
    return () => {
      clearSearch();
    };
  }, [clearSearch]);

  const renderTabContent = () => {
    switch (activeTab) {
      case "account":
        return <AccountSettings />;
      case "general":
        return <GeneralSettings />;
      case "editor":
        return <EditorSettings />;
      case "git":
        return <GitSettings />;
      case "appearance":
        return <AppearanceSettings />;
      case "databases":
        return <DatabaseSettings />;
      case "extensions":
        return <ExtensionsSettings />;
      case "ai":
        return <AISettings />;
      case "keyboard":
        return <KeyboardSettings />;
      case "language":
        return <LanguageSettings />;
      case "features":
        return <FeaturesSettings />;
      case "enterprise":
        return hasEnterpriseAccess ? <EnterpriseSettings /> : <GeneralSettings />;
      case "advanced":
        return <AdvancedSettings />;
      case "terminal":
        return <TerminalSettings />;
      case "file-explorer":
        return <FileTreeSettings />;
      default:
        return <GeneralSettings />;
    }
  };

  return (
    <div className="flex h-full w-full overflow-hidden bg-primary-bg">
      {/* Left sidebar */}
      <div className="flex w-56 shrink-0 flex-col border-r border-border/60">
        {/* Search */}
        <div className="shrink-0 border-b border-border/60 p-3">
          <Input
            type="text"
            placeholder="Search settings..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            leftIcon={Search}
            size="sm"
            className="w-full"
          />
        </div>
        <div className="min-h-0 flex-1 overflow-hidden">
          <SettingsVerticalTabs activeTab={activeTab} onTabChange={handleTabChange} />
        </div>
      </div>

      {/* Main content */}
      <div
        data-settings-content=""
        className="flex-1 overflow-y-auto p-6 [--app-ui-control-font-size:var(--ui-text-sm)] [overscroll-behavior:contain]"
      >
        {renderTabContent()}
      </div>
    </div>
  );
}
