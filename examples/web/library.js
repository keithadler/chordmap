// Recent charts, kept in this browser's IndexedDB. Charts only, never audio.
// A chart is keyed by the file's name, size and modification time so the
// same file dropped again gets its edits back.

const DB = "chordmap", STORE = "charts";
function open() {
  return new Promise((resolve, reject) => {
    const r = indexedDB.open(DB, 1);
    r.onupgradeneeded = () => { const db = r.result; if (!db.objectStoreNames.contains(STORE)) db.createObjectStore(STORE, { keyPath: "key" }).createIndex("when", "when"); };
    r.onsuccess = () => resolve(r.result);
    r.onerror = () => reject(r.error);
  });
}
function tx(mode, fn) {
  return open().then((db) => new Promise((resolve, reject) => {
    const t = db.transaction(STORE, mode), s = t.objectStore(STORE);
    const req = fn(s);
    t.oncomplete = () => resolve(req && req.result);
    t.onerror = () => reject(t.error);
  })).catch(() => undefined);
}
export const fileKey = (f) => `${f.name}|${f.size}|${f.lastModified || 0}`;
export function save(key, title, analysis) {
  return tx("readwrite", (s) => s.put({ key, title, analysis, when: Date.now() }));
}
export function load(key) { return tx("readonly", (s) => s.get(key)); }
export function remove(key) { return tx("readwrite", (s) => s.delete(key)); }
export function list() {
  return tx("readonly", (s) => s.getAll()).then((all) => (all || []).sort((a, b) => b.when - a.when).slice(0, 50));
}
