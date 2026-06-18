import { createRequire } from "module";

// `req` is never used as a real require object inside this module (no `.resolve`, no
// value/member access). It is only re-exported, so the local demand-driven analysis sees
// nothing that keeps it. It must still NOT be cleared to `undefined`, because an importer
// calls `req.resolve(...)` on the exported value — being an ESM named export keeps it.
export const req = createRequire(import.meta.url);
