import { useEffect, useState } from "react";
import { getDefaultSetting, useSettingsStore } from "@/features/settings/store";
import Section, { SettingRow } from "../settings-section";
import { controlFieldSurfaceVariants } from "@/ui/control-field";
import { cn } from "@/utils/cn";

export const FileTreeSettings = () => {
  const { settings, updateSetting } = useSettingsStore();

  const [filePatternsInput, setFilePatternsInput] = useState(
    settings.hiddenFilePatterns.join(", "),
  );
  const [directoryPatternsInput, setDirectoryPatternsInput] = useState(
    settings.hiddenDirectoryPatterns.join(", "),
  );

  useEffect(() => {
    setFilePatternsInput(settings.hiddenFilePatterns.join(", "));
  }, [settings.hiddenFilePatterns]);

  useEffect(() => {
    setDirectoryPatternsInput(settings.hiddenDirectoryPatterns.join(", "));
  }, [settings.hiddenDirectoryPatterns]);

  const parsePatterns = (input: string) =>
    input
      .split(",")
      .map((p) => p.trim())
      .filter((p) => p.length > 0);

  const commitFilePatterns = () => {
    updateSetting("hiddenFilePatterns", parsePatterns(filePatternsInput));
  };

  const commitDirectoryPatterns = () => {
    updateSetting("hiddenDirectoryPatterns", parsePatterns(directoryPatternsInput));
  };

  return (
    <div className="space-y-4">
      <Section title="Filters">
        <SettingRow
          label="Hidden Files"
          description="Comma-separated glob patterns"
          onReset={() =>
            updateSetting("hiddenFilePatterns", getDefaultSetting("hiddenFilePatterns"))
          }
          canReset={
            settings.hiddenFilePatterns.join(",") !==
            getDefaultSetting("hiddenFilePatterns").join(",")
          }
        >
          <textarea
            value={filePatternsInput}
            onChange={(e) => setFilePatternsInput(e.target.value)}
            onBlur={commitFilePatterns}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                commitFilePatterns();
              }
            }}
            placeholder="*.log, *.tmp, **/*.bak"
            rows={2}
            className={cn(
              controlFieldSurfaceVariants({ variant: "secondary" }),
              "ui-font ui-text-sm w-48 resize-none px-2 py-1.5 placeholder:text-text-lighter",
            )}
          />
        </SettingRow>

        <SettingRow
          label="Hidden Directories"
          description="Comma-separated glob patterns"
          onReset={() =>
            updateSetting("hiddenDirectoryPatterns", getDefaultSetting("hiddenDirectoryPatterns"))
          }
          canReset={
            settings.hiddenDirectoryPatterns.join(",") !==
            getDefaultSetting("hiddenDirectoryPatterns").join(",")
          }
        >
          <textarea
            value={directoryPatternsInput}
            onChange={(e) => setDirectoryPatternsInput(e.target.value)}
            onBlur={commitDirectoryPatterns}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                commitDirectoryPatterns();
              }
            }}
            placeholder="node_modules, .git, build/"
            rows={2}
            className={cn(
              controlFieldSurfaceVariants({ variant: "secondary" }),
              "ui-font ui-text-sm w-48 resize-none px-2 py-1.5 placeholder:text-text-lighter",
            )}
          />
        </SettingRow>
      </Section>
    </div>
  );
};
