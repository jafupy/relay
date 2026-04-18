import { extensionManager } from "@/features/editor/extensions/manager";
import { invoke } from "@/lib/platform/core";
import { themeLoader } from "./theme-loader";
import { themeRegistry } from "./theme-registry";

let isThemeSystemInitialized = false;

const rebuildNativeMenu = async () => {
  try {
    const themes = themeRegistry.getAllThemes();
    const themeData = themes.map((theme) => ({
      id: theme.id,
      name: theme.name,
      category: theme.category,
    }));

    await invoke("rebuild_menu_themes", { themes: themeData });
  } catch (error) {
    console.error("Failed to rebuild native menu:", error);
  }
};

export const initializeThemeSystem = async () => {
  if (isThemeSystemInitialized) {
    return;
  }

  try {
    isThemeSystemInitialized = true;

    if (!extensionManager.isInitialized()) {
      extensionManager.initialize();
    }

    const dummyEditorAPI = {
      getContent: () => "",
      setContent: () => {},
      insertText: () => {},
      deleteRange: () => {},
      replaceRange: () => {},
      getSelection: () => null,
      setSelection: () => {},
      getCursorPosition: () => ({ line: 0, column: 0, offset: 0 }),
      setCursorPosition: () => {},
      selectAll: () => {},
      addDecoration: () => "",
      removeDecoration: () => {},
      updateDecoration: () => {},
      clearDecorations: () => {},
      getLines: () => [],
      getLine: () => undefined,
      getLineCount: () => 0,
      duplicateLine: () => {},
      deleteLine: () => {},
      toggleComment: () => {},
      moveLineUp: () => {},
      moveLineDown: () => {},
      copyLineUp: () => {},
      copyLineDown: () => {},
      undo: () => {},
      redo: () => {},
      canUndo: () => false,
      canRedo: () => false,
      getSettings: () => ({
        fontSize: 14,
        tabSize: 2,
        lineNumbers: true,
        wordWrap: false,
        theme: "relay-dark",
      }),
      updateSettings: () => {},
      on: () => () => {},
      off: () => {},
      emitEvent: () => {},
    };

    extensionManager.setEditor(dummyEditorAPI);

    try {
      await extensionManager.loadExtension(themeLoader);
    } catch (error) {
      console.error("initializeThemeSystem: Failed to load themes:", error);
    }

    themeRegistry.markAsReady();

    await rebuildNativeMenu();

    themeRegistry.onRegistryChange(() => {
      rebuildNativeMenu();
    });
  } catch (error) {
    console.error("Failed to initialize theme system:", error);
    isThemeSystemInitialized = false;
  }
};
