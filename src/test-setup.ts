import "@testing-library/jest-dom/vitest";

// Node's experimental native `localStorage` global is undefined unless
// `--localstorage-file` is passed, and it shadows jsdom's window.localStorage.
// Provide a working in-memory implementation so components that read/write
// localStorage (e.g. the theme provider) don't crash during tests.
function createMemoryStorage(): Storage {
  const store = new Map<string, string>();
  return {
    get length() {
      return store.size;
    },
    clear: () => store.clear(),
    getItem: (key: string) => (store.has(key) ? store.get(key)! : null),
    key: (index: number) => Array.from(store.keys())[index] ?? null,
    removeItem: (key: string) => {
      store.delete(key);
    },
    setItem: (key: string, value: string) => {
      store.set(key, String(value));
    },
  } as Storage;
}

const memoryStorage = createMemoryStorage();
for (const target of [globalThis, window]) {
  Object.defineProperty(target, "localStorage", {
    writable: true,
    configurable: true,
    value: memoryStorage,
  });
}

Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  }),
});
