import type { EditorAPI } from "@/features/editor/extensions/types";
import { BaseThemeExtension } from "./base-theme-extension";
import ayuThemes from "./builtin/ayu.json";
import catppuccinThemes from "./builtin/catppuccin.json";
import christmasThemes from "./builtin/christmas.json";
import contrastThemes from "./builtin/contrast-themes.json";
import draculaThemes from "./builtin/dracula.json";
import githubThemes from "./builtin/github.json";
import nordThemes from "./builtin/nord.json";
import oneThemes from "./builtin/one.json";
import relayThemes from "./builtin/relay.json";
import solarizedThemes from "./builtin/solarized.json";
import tokyoNightThemes from "./builtin/tokyo-night.json";
import vitesseThemes from "./builtin/vitesse.json";
import type { ThemeDefinition } from "./types";

interface JsonTheme {
  id: string;
  name: string;
  description?: string;
  appearance: "dark" | "light";
  colors: Record<string, string>;
  syntax: Record<string, string>;
}

interface ThemeFile {
  name: string;
  author?: string;
  description?: string;
  themes: JsonTheme[];
}

export class ThemeLoader extends BaseThemeExtension {
  readonly name = "Theme Loader";
  readonly version = "1.0.0";
  readonly description = "Loads themes from JSON configuration files";
  themes: ThemeDefinition[] = [];

  async onInitialize(_editor: EditorAPI): Promise<void> {
    try {
      const allThemeFiles: ThemeFile[] = [
        ayuThemes as ThemeFile,
        relayThemes as ThemeFile,
        catppuccinThemes as ThemeFile,
        christmasThemes as ThemeFile,
        contrastThemes as ThemeFile,
        draculaThemes as ThemeFile,
        githubThemes as ThemeFile,
        nordThemes as ThemeFile,
        oneThemes as ThemeFile,
        solarizedThemes as ThemeFile,
        tokyoNightThemes as ThemeFile,
        vitesseThemes as ThemeFile,
      ];

      const allThemes: JsonTheme[] = allThemeFiles.flatMap((file) => file.themes);

      this.themes = allThemes.map((jsonTheme) => this.convertJsonToThemeDefinition(jsonTheme));

      const { themeRegistry } = await import("./theme-registry");
      this.themes.forEach((theme) => {
        themeRegistry.registerTheme(theme);
      });
    } catch (error) {
      console.error("ThemeLoader: Failed to load JSON themes:", error);
      this.themes = [];
    }
  }

  private convertJsonToThemeDefinition(jsonTheme: JsonTheme): ThemeDefinition {
    const cssVariables: Record<string, string> = {};
    for (const [key, value] of Object.entries(jsonTheme.colors)) {
      cssVariables[`--${key}`] = value;
      cssVariables[`--color-${key}`] = value;
    }

    const syntaxTokens: Record<string, string> = {};
    for (const [key, value] of Object.entries(jsonTheme.syntax)) {
      syntaxTokens[`--syntax-${key}`] = value;
      syntaxTokens[`--color-syntax-${key}`] = value;
    }

    const isDark = jsonTheme.appearance === "dark";

    return {
      id: jsonTheme.id,
      name: jsonTheme.name,
      description: jsonTheme.description || "",
      category: isDark ? "Dark" : "Light",
      cssVariables,
      syntaxTokens,
      isDark,
      icon: undefined,
    };
  }

  async loadFromFile(filePath: string): Promise<ThemeDefinition[]> {
    try {
      const response = await fetch(filePath);
      if (!response.ok) {
        throw new Error(`Failed to fetch theme file: ${response.statusText}`);
      }

      const themeFile: ThemeFile = await response.json();
      return themeFile.themes.map((jsonTheme) => this.convertJsonToThemeDefinition(jsonTheme));
    } catch (error) {
      console.error(`ThemeLoader: Failed to load theme from ${filePath}:`, error);
      return [];
    }
  }

  async getCachedThemes(): Promise<ThemeDefinition[]> {
    return this.themes;
  }
}

export const themeLoader = new ThemeLoader();
