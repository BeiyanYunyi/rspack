import { req } from "./dep";

it("keeps an exported created require so cross-module `.resolve` works (requireResolve disabled)", () => {
	// If `dep.js`'s createRequire had been cleared to `undefined`, this would throw
	// `Cannot read properties of undefined`.
	expect(typeof req).toBe("function");
	// `.resolve` runs at runtime: a builtin resolves to its own name.
	expect(req.resolve("path")).toBe("path");
});
