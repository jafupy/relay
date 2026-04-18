import type { EditorAPI } from "@/features/editor/extensions/types";
import { themeRegistry } from "./theme-registry";
import type { ThemeDefinition, ThemeExtension } from "./types";

export abstract class BaseThemeExtension implements ThemeExtension {
  readonly extensionType = "theme" as const;
  abstract readonly name: string;
  abstract readonly version: string;
  abstract readonly description: string;
  abstract readonly themes: ThemeDefinition[];

  private registeredThemes = new Set<string>();

  async initialize(editor: EditorAPI): Promise<void> {
    this.themes.forEach((theme) => {
      themeRegistry.registerTheme(theme);
      this.registeredThemes.add(theme.id);
    });

    await this.onInitialize?.(editor);
  }

  dispose(): void {
    this.registeredThemes.forEach((themeId) => {
      themeRegistry.unregisterTheme(themeId);
    });
    this.registeredThemes.clear();

    this.onDispose?.();
  }

  getTheme(id: string): ThemeDefinition | undefined {
    return this.themes.find((theme) => theme.id === id);
  }

  applyTheme(id: string): void {
    themeRegistry.applyTheme(id);
  }

  removeTheme(id: string): void {
    themeRegistry.unregisterTheme(id);
    this.registeredThemes.delete(id);
  }

  protected onInitialize?(editor: EditorAPI): Promise<void> | void;
  protected onDispose?(): void;
}
