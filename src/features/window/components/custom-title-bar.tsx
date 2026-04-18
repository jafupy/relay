import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { useBufferStore } from "@/features/editor/stores/buffer-store";
import { useFileSystemStore } from "@/features/file-system/controllers/store";
import { SidebarPaneSelector } from "@/features/layout/components/sidebar/sidebar-pane-selector";
import {
  resolveSidebarPaneClick,
  type SidebarView,
} from "@/features/layout/utils/sidebar-pane-utils";
import { useSettingsStore } from "@/features/settings/store";
import { useUIState } from "@/features/window/stores/ui-state-store";
import { useWorkspaceTabsStore } from "@/features/window/stores/workspace-tabs-store";
import { ContextMenu, type ContextMenuItem, useContextMenu } from "@/ui/context-menu";
import { IS_MAC } from "@/utils/platform";
import { AccountMenu } from "./account-menu";
import CustomMenuBar from "./menu-bar/window-menu-bar";
import { NotificationsMenu } from "./notifications-menu";
import ProjectTabs from "./project-tabs";
import RunActionsButton from "./run-actions-button";
import WindowTitleDisplay from "./window-title-display";

interface CustomTitleBarProps {
  title?: string;
  showMinimal?: boolean;
}

const CustomTitleBar = ({ showMinimal = false }: CustomTitleBarProps) => {
  const { settings } = useSettingsStore();
  const handleOpenFolder = useFileSystemStore((state) => state.handleOpenFolder);
  const projectTabs = useWorkspaceTabsStore.use.projectTabs();
  const {
    isGitViewActive,
    isGitHubPRsViewActive,
    activeSidebarView,
    isSidebarVisible,
    setActiveView,
    setIsSidebarVisible,
    setIsGlobalSearchVisible,
    isGlobalSearchVisible,
    setIsProjectPickerVisible,
  } = useUIState();

  const [menuBarActiveMenu, setMenuBarActiveMenu] = useState<string | null>(null);
  const titleBarContextMenu = useContextMenu();
  const titleBarProjectMode = settings.titleBarProjectMode;

  const handleSidebarViewChange = (view: SidebarView) => {
    const { nextIsSidebarVisible, nextView } = resolveSidebarPaneClick(
      { isSidebarVisible, isGitViewActive, isGitHubPRsViewActive },
      view,
    );
    setActiveView(nextView);
    setIsSidebarVisible(nextIsSidebarVisible);
  };

  const handleTitleBarContextMenu = (e: React.MouseEvent<HTMLDivElement>) => {
    const target = e.target as HTMLElement;
    const interactiveTarget = target.closest(
      "button, a, input, textarea, select, [role='tab'], [data-title-bar-project-tabs='true'], [contenteditable='true']",
    );
    if (interactiveTarget) return;
    titleBarContextMenu.open(e);
  };

  const titleBarContextMenuItems: ContextMenuItem[] = [
    {
      id: "add-project",
      label: "Add Project",
      onClick: () => setIsProjectPickerVisible(true),
    },
    {
      id: "open-project",
      label: "Open Folder",
      onClick: () => {
        void handleOpenFolder();
      },
    },
    ...(projectTabs.length > 0
      ? [
          { id: "sep-projects", label: "", separator: true, onClick: () => {} },
          {
            id: "close-all-projects",
            label: "Close All Projects",
            onClick: () => {
              useWorkspaceTabsStore.getState().closeAllProjectTabs();
            },
          },
        ]
      : []),
  ];

  const titleBarContextMenuPortal = createPortal(
    <ContextMenu
      isOpen={titleBarContextMenu.isOpen}
      position={titleBarContextMenu.position}
      items={titleBarContextMenuItems}
      onClose={titleBarContextMenu.close}
    />,
    document.body,
  );

  if (showMinimal) {
    return (
      <div
        data-relay-drag-region
        className="relative z-50 flex h-9 select-none items-center bg-primary-bg border-b border-border/30 px-3"
      />
    );
  }

  return (
    <div
      data-relay-drag-region
      onContextMenu={handleTitleBarContextMenu}
      className="relative z-50 flex h-9 select-none items-center justify-between bg-primary-bg border-b border-border/30 pl-2 pr-2"
    >
      {/* Left: inline menu + sidebar icons */}
      <div className="pointer-events-auto flex h-9 min-w-0 items-center gap-2">
        {!settings.nativeMenuBar && (
          <CustomMenuBar activeMenu={menuBarActiveMenu} setActiveMenu={setMenuBarActiveMenu} />
        )}

        <div className="h-4 w-px shrink-0 bg-border/40" />

        <SidebarPaneSelector
          activeSidebarView={activeSidebarView}
          isGitViewActive={isGitViewActive}
          isGitHubPRsViewActive={isGitHubPRsViewActive}
          coreFeatures={settings.coreFeatures}
          onViewChange={handleSidebarViewChange}
          onSearchClick={() => setIsGlobalSearchVisible(!isGlobalSearchVisible)}
          compact
        />
      </div>

      {/* Center: project tabs (absolute so it doesn't affect flex layout) */}
      <div className="pointer-events-none absolute inset-x-0 top-0 flex h-9 justify-center">
        <div
          data-title-bar-project-tabs="true"
          className="pointer-events-auto flex h-9 items-center"
        >
          {titleBarProjectMode === "window" ? <WindowTitleDisplay /> : <ProjectTabs />}
        </div>
      </div>

      {/* Right: actions + account */}
      <div className="flex h-9 items-center gap-0.5">
        <RunActionsButton />
        <NotificationsMenu iconSize={13} />
        <AccountMenu iconSize={13} />
      </div>

      {titleBarContextMenuPortal}
    </div>
  );
};

const CustomTitleBarWithSettings = (props: CustomTitleBarProps) => {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "," && (IS_MAC ? e.metaKey : e.ctrlKey)) {
        e.preventDefault();
        useBufferStore.getState().actions.openContent({ type: "settings" });
      }
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, []);

  return <CustomTitleBar {...props} />;
};

export default CustomTitleBarWithSettings;
