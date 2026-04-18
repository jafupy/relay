type WebviewWindowEvent = { payload: any };
type WebviewWindowEventHandler = (event: WebviewWindowEvent) => void | Promise<void>;

export function getCurrentWebviewWindow() {
  return {
    label: "main",
    listen: async (_event: string, _handler: WebviewWindowEventHandler) => () => {},
    emit: async (_event: string, _payload?: unknown) => {},
    setTitle: async (_title?: string) => {},
  };
}
