const noop = async () => {};

export type ResizeDirection =
  | "East"
  | "North"
  | "NorthEast"
  | "NorthWest"
  | "South"
  | "SouthEast"
  | "SouthWest"
  | "West";

type WindowEvent = { payload: any };
type WindowEventHandler = (event: WindowEvent) => void | Promise<void>;

export function getCurrentWindow() {
  return {
    minimize: noop,
    maximize: noop,
    unmaximize: noop,
    close: () => window.close(),
    startDragging: noop,
    startResizeDragging: async (_direction: ResizeDirection) => {},
    toggleMaximize: noop,
    isMaximized: async () => false,
    isFullscreen: async () => false,
    setFullscreen: async (_fullscreen: boolean) => {},
    listen: async (_event: string, _handler: WindowEventHandler) => () => {},
    onResized: async (_handler: WindowEventHandler) => () => {},
    onFocusChanged: async (_handler: WindowEventHandler) => () => {},
    onDragDropEvent: async (_handler: WindowEventHandler) => () => {},
  };
}
