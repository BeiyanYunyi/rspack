import { createRequire } from "module";

const r = createRequire(import.meta.url);

// Value-position use: passing the created require to a function must keep the real
// createRequire (a webpack context-require could not resolve a builtin).
function id(x) {
	return x;
}
export const passed = id(r);

it("keeps a real created require when accessed by member or value (requireResolve disabled)", () => {
	// The value passed out is the real created require, not a context require.
	expect(typeof passed).toBe("function");
	expect(passed.resolve("path")).toBe("path");

	// A non-resolve member access reaches the real createRequire (Node require.resolve.paths),
	// instead of being rewritten to an unsupported `undefined`.
	expect(typeof r.resolve.paths).toBe("function");

	// Invoking it is still bundled.
	expect(r("./dep")).toBe(1);

	// And `.resolve` itself is still preserved at runtime.
	expect(r.resolve("path")).toBe("path");
});
