it("should work import.meta.env with EnvironmentPlugin", () => {
    expect(import.meta.env.AAA).toBe(process.env.AAA);
});

it("import.meta.env behaves like process.env", () => {
    try {
        const importMetaEnv = import.meta.env;
        importMetaEnv;
        const processEnv = process.env;
        processEnv;
        const UNKNOWN_PROPERTY = import.meta.env.UNKNOWN_PROPERTY;
        UNKNOWN_PROPERTY;
        const UNKNOWN_PROPERTY_2 = process.env.UNKNOWN_PROPERTY_2;
        UNKNOWN_PROPERTY_2;
        typeof import.meta.env;
        typeof process.env;

        const { env } = import.meta;
        env;
    } catch (_e) {
        // ignore
    }
});
