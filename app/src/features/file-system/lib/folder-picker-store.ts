import { create } from "zustand";

interface FolderPickerState {
  isOpen: boolean;
  _resolve: ((path: string | null) => void) | null;
}

interface FolderPickerActions {
  openPicker: () => Promise<string | null>;
  confirm: (path: string) => void;
  cancel: () => void;
}

export const useFolderPickerStore = create<FolderPickerState & FolderPickerActions>((set, get) => ({
  isOpen: false,
  _resolve: null,

  openPicker: () => {
    return new Promise<string | null>((resolve) => {
      set({ isOpen: true, _resolve: resolve });
    });
  },

  confirm: (path: string) => {
    const { _resolve } = get();
    set({ isOpen: false, _resolve: null });
    _resolve?.(path);
  },

  cancel: () => {
    const { _resolve } = get();
    set({ isOpen: false, _resolve: null });
    _resolve?.(null);
  },
}));
