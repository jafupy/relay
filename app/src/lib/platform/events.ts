export type UnlistenFn = () => void;

type Listener<T> = (event: { event: string; payload: T }) => void;

const listeners = new Map<string, Set<Listener<unknown>>>();
let socket: WebSocket | null = null;

export async function listen<T>(event: string, handler: Listener<T>): Promise<UnlistenFn> {
  ensureSocket();
  const set = listeners.get(event) ?? new Set<Listener<unknown>>();
  set.add(handler as Listener<unknown>);
  listeners.set(event, set);

  return () => {
    set.delete(handler as Listener<unknown>);
    if (set.size === 0) {
      listeners.delete(event);
    }
  };
}

function ensureSocket() {
  if (socket && socket.readyState !== WebSocket.CLOSED) return;
  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  socket = new WebSocket(`${protocol}//${window.location.host}/api/events`);
  socket.addEventListener("message", (message) => {
    const event = JSON.parse(message.data) as { event: string; payload: unknown };
    for (const handler of listeners.get(event.event) ?? []) {
      handler(event);
    }
  });
}
