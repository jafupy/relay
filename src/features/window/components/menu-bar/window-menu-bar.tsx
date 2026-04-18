import type React from "react";
import { useEffect, useMemo, useRef, useState } from "react";
import { themeRegistry } from "@/extensions/themes/theme-registry";
import type { ThemeDefinition } from "@/extensions/themes/types";
import { useBufferStore } from "@/features/editor/stores/buffer-store";
import { getCurrentWebviewWindow } from "@/lib/platform/webview-window";
import { cn } from "@/utils/cn";
import Menu from "./menu";
import MenuItem from "./menu-item";
import Submenu from "./submenu";

interface Props {
  activeMenu: string | null;
  setActiveMenu: React.Dispatch<React.SetStateAction<string | null>>;
}

const CustomMenuBar = ({ activeMenu, setActiveMenu }: Props) => {
  const [themes, setThemes] = useState<ThemeDefinition[]>([]);
  const menuBarRef = useRef<HTMLDivElement>(null);

  const emit = (event: string, payload?: unknown) => {
    void getCurrentWebviewWindow().emit(event, payload);
    setActiveMenu(null);
  };

  useEffect(() => {
    const load = () => setThemes(themeRegistry.getAllThemes());
    load();
    return themeRegistry.onRegistryChange(load);
  }, []);

  const menus = useMemo(
    () => ({
      File: (
        <Menu aria-label="File">
          <MenuItem onClick={() => emit("menu_new_file")}>New File</MenuItem>
          <MenuItem shortcut="mod+o" onClick={() => emit("menu_open_folder")}>
            Open Folder
          </MenuItem>
          <MenuItem onClick={() => emit("menu_close_folder")}>Close Folder</MenuItem>
          <MenuItem separator />
          <MenuItem shortcut="mod+s" onClick={() => emit("menu_save")}>
            Save
          </MenuItem>
          <MenuItem shortcut="mod+shift+s" onClick={() => emit("menu_save_as")}>
            Save As...
          </MenuItem>
          <MenuItem separator />
          <MenuItem shortcut="mod+w" onClick={() => emit("menu_close_tab")}>
            Close Tab
          </MenuItem>
        </Menu>
      ),
      Edit: (
        <Menu aria-label="Edit">
          <MenuItem shortcut="mod+z" onClick={() => emit("menu_undo")}>
            Undo
          </MenuItem>
          <MenuItem shortcut="mod+shift+z" onClick={() => emit("menu_redo")}>
            Redo
          </MenuItem>
          <MenuItem separator />
          <MenuItem shortcut="mod+x">Cut</MenuItem>
          <MenuItem shortcut="mod+c">Copy</MenuItem>
          <MenuItem shortcut="mod+v">Paste</MenuItem>
          <MenuItem shortcut="mod+a">Select All</MenuItem>
          <MenuItem separator />
          <MenuItem shortcut="mod+f" onClick={() => emit("menu_find")}>
            Find
          </MenuItem>
          <MenuItem shortcut="mod+alt+f" onClick={() => emit("menu_find_replace")}>
            Find and Replace
          </MenuItem>
          <MenuItem separator />
          <MenuItem shortcut="mod+shift+p" onClick={() => emit("menu_command_palette")}>
            Command Palette
          </MenuItem>
        </Menu>
      ),
      View: (
        <Menu aria-label="View">
          <MenuItem shortcut="mod+b" onClick={() => emit("menu_toggle_sidebar")}>
            Toggle Sidebar
          </MenuItem>
          <MenuItem shortcut="mod+j" onClick={() => emit("menu_toggle_terminal")}>
            Toggle Terminal
          </MenuItem>
          <MenuItem shortcut="mod+r" onClick={() => emit("menu_toggle_ai_chat")}>
            Toggle AI Chat
          </MenuItem>
          <MenuItem separator />
          <MenuItem onClick={() => emit("menu_split_editor")}>Split Editor</MenuItem>
          <MenuItem separator />
          <Submenu title="Theme">
            {themes.map((theme) => (
              <MenuItem key={theme.id} onClick={() => emit("menu_theme_change", theme.id)}>
                {theme.name}
              </MenuItem>
            ))}
          </Submenu>
        </Menu>
      ),
      Go: (
        <Menu aria-label="Go">
          <MenuItem shortcut="mod+p" onClick={() => emit("menu_quick_open")}>
            Quick Open
          </MenuItem>
          <MenuItem shortcut="mod+g" onClick={() => emit("menu_go_to_line")}>
            Go to Line
          </MenuItem>
          <MenuItem separator />
          <MenuItem shortcut="mod+alt+right" onClick={() => emit("menu_next_tab")}>
            Next Tab
          </MenuItem>
          <MenuItem shortcut="mod+alt+left" onClick={() => emit("menu_prev_tab")}>
            Previous Tab
          </MenuItem>
        </Menu>
      ),
      Help: (
        <Menu aria-label="Help">
          <MenuItem onClick={() => emit("menu_help")}>Help</MenuItem>
          <MenuItem separator />
          <MenuItem
            onClick={() => {
              useBufferStore.getState().actions.openContent({ type: "settings" });
              setActiveMenu(null);
            }}
          >
            Settings
          </MenuItem>
          <MenuItem separator />
          <MenuItem onClick={() => emit("menu_about_relay")}>About Relay</MenuItem>
        </Menu>
      ),
    }),
    [themes, emit, setActiveMenu],
  );

  useEffect(() => {
    if (!activeMenu) return;
    const handleMouseDown = (e: MouseEvent) => {
      if (menuBarRef.current && !menuBarRef.current.contains(e.target as Node)) {
        setActiveMenu(null);
      }
    };
    document.addEventListener("mousedown", handleMouseDown);
    return () => document.removeEventListener("mousedown", handleMouseDown);
  }, [activeMenu, setActiveMenu]);

  return (
    <div ref={menuBarRef} className="flex items-center gap-px">
      {Object.keys(menus).map((name) => (
        <div key={name} className="relative">
          <button
            type="button"
            className={cn(
              "select-none rounded-md px-2 py-1 text-[11px] font-medium tracking-wide transition-colors",
              activeMenu === name
                ? "bg-hover text-text"
                : "text-text-lighter hover:bg-hover/60 hover:text-text",
            )}
            onClick={() => setActiveMenu((cur) => (cur === name ? null : name))}
          >
            {name}
          </button>
          {activeMenu === name && (
            <div className="absolute left-0 top-full z-[10030] pt-1">
              {menus[name as keyof typeof menus]}
            </div>
          )}
        </div>
      ))}
    </div>
  );
};

export default CustomMenuBar;
