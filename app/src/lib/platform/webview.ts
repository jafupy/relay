export function getCurrentWebview() {
  return {
    onDragDropEvent:
      async (_handler?: (event: { payload: any }) => void | Promise<void>) => () => {},
  };
}
