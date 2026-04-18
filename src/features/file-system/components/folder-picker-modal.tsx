import { ChevronLeft, ChevronRight, Folder, FolderOpen, HardDrive, Home } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { readDirectory } from "@/features/file-system/controllers/platform";
import { useFolderPickerStore } from "@/features/file-system/lib/folder-picker-store";
import { Button } from "@/ui/button";
import Input from "@/ui/input";
import { cn } from "@/utils/cn";
import { IS_MAC } from "@/utils/platform";

interface DirEntry {
  name: string;
  path: string;
  is_dir: boolean;
}

function getParentPath(path: string): string | null {
  const sep = path.includes("\\") ? "\\" : "/";
  const parts = path.replace(/[/\\]+$/, "").split(sep);
  if (parts.length <= 1) return null;
  const parent = parts.slice(0, -1).join(sep);
  return parent || sep;
}

function getDefaultStartPath(): string {
  if (IS_MAC) return "/Users";
  return "/home";
}

export function FolderPickerModal() {
  const { isOpen, confirm, cancel } = useFolderPickerStore();

  const [currentPath, setCurrentPath] = useState(getDefaultStartPath());
  const [inputPath, setInputPath] = useState(getDefaultStartPath());
  const [entries, setEntries] = useState<DirEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [history, setHistory] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const inputRef = useRef<HTMLInputElement>(null);

  const loadDirectory = useCallback(
    async (path: string, pushHistory = true) => {
      setLoading(true);
      setError(null);
      setSelectedPath(null);
      try {
        const raw = await readDirectory(path);
        const dirs = (raw as DirEntry[])
          .filter((e) => e.is_dir)
          .sort((a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: "base" }));
        setEntries(dirs);
        setCurrentPath(path);
        setInputPath(path);
        if (pushHistory) {
          setHistory((prev) => {
            const trimmed = prev.slice(0, historyIndex + 1);
            return [...trimmed, path];
          });
          setHistoryIndex((prev) => prev + 1);
        }
      } catch {
        setError(`Cannot read directory: ${path}`);
        setEntries([]);
      } finally {
        setLoading(false);
      }
    },
    [historyIndex],
  );

  // Reset and load when opened
  useEffect(() => {
    if (isOpen) {
      const start = getDefaultStartPath();
      setCurrentPath(start);
      setInputPath(start);
      setHistory([start]);
      setHistoryIndex(0);
      setSelectedPath(null);
      setEntries([]);
      setError(null);
      loadDirectory(start, false);
    }
  }, [isOpen]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleNavigate = (path: string) => {
    void loadDirectory(path);
  };

  const handleInputKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      void loadDirectory(inputPath);
    }
    if (e.key === "Escape") {
      cancel();
    }
  };

  const handleBack = () => {
    if (historyIndex > 0) {
      const newIndex = historyIndex - 1;
      const path = history[newIndex];
      setHistoryIndex(newIndex);
      void loadDirectory(path, false);
    }
  };

  const handleForward = () => {
    if (historyIndex < history.length - 1) {
      const newIndex = historyIndex + 1;
      const path = history[newIndex];
      setHistoryIndex(newIndex);
      void loadDirectory(path, false);
    }
  };

  const handleUp = () => {
    const parent = getParentPath(currentPath);
    if (parent) void loadDirectory(parent);
  };

  const handleHome = () => {
    const home = IS_MAC ? "/Users" : "/home";
    void loadDirectory(home);
  };

  const handleSelect = () => {
    confirm(selectedPath ?? currentPath);
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-[10050] flex items-center justify-center bg-black/60 backdrop-blur-sm">
      <div
        className="flex w-[640px] max-w-[90vw] flex-col overflow-hidden rounded-2xl border border-border bg-primary-bg shadow-2xl"
        style={{ height: "480px" }}
      >
        {/* Header */}
        <div className="flex shrink-0 items-center gap-2 border-b border-border px-4 py-3">
          <HardDrive className="size-4 shrink-0 text-text-lighter" />
          <span className="text-sm font-medium text-text">Open Folder</span>
        </div>

        {/* Toolbar */}
        <div className="flex shrink-0 items-center gap-1 border-b border-border/60 px-3 py-2">
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={handleBack}
            disabled={historyIndex <= 0}
            className="shrink-0"
          >
            <ChevronLeft className="size-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={handleForward}
            disabled={historyIndex >= history.length - 1}
            className="shrink-0"
          >
            <ChevronRight className="size-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={handleUp}
            disabled={!getParentPath(currentPath)}
            className="shrink-0"
          >
            <ChevronLeft className="size-3 rotate-90" />
          </Button>
          <Button variant="ghost" size="icon-sm" onClick={handleHome} className="shrink-0">
            <Home className="size-4" />
          </Button>

          <div className="mx-1 h-4 w-px bg-border/60 shrink-0" />

          <Input
            ref={inputRef}
            value={inputPath}
            onChange={(e) => setInputPath(e.target.value)}
            onKeyDown={handleInputKeyDown}
            className="h-7 flex-1 text-xs"
            placeholder="Path..."
          />
        </div>

        {/* Directory listing */}
        <div className="min-h-0 flex-1 overflow-y-auto p-2">
          {loading ? (
            <div className="flex h-full items-center justify-center text-sm text-text-lighter">
              Loading...
            </div>
          ) : error ? (
            <div className="flex h-full items-center justify-center text-sm text-red-400">
              {error}
            </div>
          ) : entries.length === 0 ? (
            <div className="flex h-full items-center justify-center text-sm text-text-lighter">
              Empty directory
            </div>
          ) : (
            entries.map((entry) => (
              <button
                key={entry.path}
                type="button"
                className={cn(
                  "flex w-full items-center gap-2.5 rounded-lg px-2.5 py-1.5 text-left text-sm transition-colors",
                  selectedPath === entry.path
                    ? "bg-accent/15 text-accent"
                    : "text-text hover:bg-hover/60",
                )}
                onClick={() => setSelectedPath(entry.path)}
                onDoubleClick={() => handleNavigate(entry.path)}
              >
                {selectedPath === entry.path ? (
                  <FolderOpen className="size-4 shrink-0 text-accent" />
                ) : (
                  <Folder className="size-4 shrink-0 text-text-lighter" />
                )}
                <span className="truncate">{entry.name}</span>
              </button>
            ))
          )}
        </div>

        {/* Current selection bar */}
        <div className="shrink-0 border-t border-border/60 bg-secondary-bg/40 px-4 py-2">
          <div className="truncate text-xs text-text-lighter">
            <span className="text-text-lighter">Selected: </span>
            <span className="font-medium text-text">{selectedPath ?? currentPath}</span>
          </div>
        </div>

        {/* Footer */}
        <div className="flex shrink-0 items-center justify-end gap-2 border-t border-border px-4 py-3">
          <Button variant="outline" size="sm" onClick={cancel}>
            Cancel
          </Button>
          <Button variant="default" size="sm" onClick={handleSelect}>
            Open Folder
          </Button>
        </div>
      </div>
    </div>
  );
}
