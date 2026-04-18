/**
 * Reactive lookup for the active keybinding of a command.
 *
 * Composes the user override (from `useKeymapStore`) with the default
 * (from `keymapRegistry`). The selector returns a primitive string so
 * reference equality short-circuits re-renders in the common case.
 */

import { useKeymapStore } from "../stores/store";
import { keymapRegistry } from "../utils/registry";

export function useCommandShortcut(commandId?: string): string | undefined {
  const userBinding = useKeymapStore((state) =>
    commandId
      ? state.keybindings.find((kb) => kb.command === commandId && kb.source === "user")?.key
      : undefined,
  );

  if (!commandId) return undefined;
  if (userBinding) return userBinding;
  return keymapRegistry.getKeybinding(commandId)?.key;
}
