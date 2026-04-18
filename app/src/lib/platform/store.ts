type StoredValue = unknown;

export interface Store {
  get<T = StoredValue>(key: string): Promise<T | null>;
  set(key: string, value: StoredValue): Promise<void>;
  delete(key: string): Promise<void>;
  save(): Promise<void>;
}

interface LoadOptions {
  autoSave?: boolean;
}

export async function load(name: string, _options?: LoadOptions): Promise<Store> {
  const prefix = `relay.store.${name}.`;
  return {
    async get<T>(key: string) {
      const value = localStorage.getItem(prefix + key);
      return value === null ? null : (JSON.parse(value) as T);
    },
    async set(key, value) {
      localStorage.setItem(prefix + key, JSON.stringify(value));
    },
    async delete(key) {
      localStorage.removeItem(prefix + key);
    },
    async save() {},
  };
}
