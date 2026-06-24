import "./tla.js"
const a = import("./defer.js")
import.meta.hot.accept(["./defer.js"], () => {})
it("should compile", async () => {
	expect(await a).toBeTruthy();
});
