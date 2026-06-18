const fs = require("fs");
const path = require("path");

let emittedSource = "";

module.exports = {
	findBundle(i, options) {
		emittedSource = fs.readFileSync(
			path.join(options.output.path, `bundle${i}.mjs`),
			"utf-8"
		);
		// `__custom_import_meta` is not defined in this environment, so don't execute.
		return [];
	},
	afterExecute() {
		// The kept createRequire honors the customized importMetaName.
		expect(emittedSource).toContain("req = __rspack_createRequire(__custom_import_meta.url)");
	}
};
