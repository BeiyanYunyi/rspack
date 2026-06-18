import { createRequire } from "module";

// requireResolve is disabled, so this created require is kept and rendered via rspack's
// `__rspack_createRequire` helper. The kept argument must honor a customized
// output.importMetaName instead of emitting a literal `import.meta`.
const req = createRequire(import.meta.url);
export const resolved = req.resolve("path");
