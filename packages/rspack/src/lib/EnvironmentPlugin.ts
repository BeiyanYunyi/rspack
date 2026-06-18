/**
 * The following code is modified based on
 * https://github.com/webpack/webpack/blob/4b4ca3b/lib/EnvironmentPlugin.js
 *
 * MIT Licensed
 * Author Tobias Koppers @sokra
 * Copyright (c) JS Foundation and other contributors
 * https://github.com/webpack/webpack/blob/main/LICENSE
 */

import type { Compiler } from '../Compiler';
import { DefinePlugin } from '../builtin-plugin';
import WebpackError from './WebpackError';

class EnvironmentPlugin {
  keys: string[];
  defaultValues: Record<string, string | undefined | null>;

  constructor(
    ...keys:
      | string[]
      | [Record<string, string | undefined | null> | string | string[]]
  ) {
    if (keys.length === 1 && Array.isArray(keys[0])) {
      this.keys = keys[0];
      this.defaultValues = {};
    } else if (keys.length === 1 && keys[0] && typeof keys[0] === 'object') {
      this.keys = Object.keys(keys[0]);
      this.defaultValues = keys[0] as Record<string, string | undefined | null>;
    } else {
      this.keys = keys as string[];
      this.defaultValues = {};
    }
  }

  /**
   * Apply the plugin
   * @param compiler the compiler instance
   * @returns
   */
  apply(compiler: Compiler) {
    const definitions: Record<string, string> = Object.create(null);
    for (const key of this.keys) {
      // Use `hasOwnProperty` rather than `process.env[key] !== undefined` so that
      // names inherited from `Object.prototype` (e.g. `__proto__`, `constructor`)
      // are not mistaken for defined env variables — `process.env.__proto__`
      // returns the prototype object, not an env value. For real env variables
      // (which are always strings) this is equivalent to the `!== undefined`
      // check, including the empty-string case.
      const value = Object.prototype.hasOwnProperty.call(process.env, key)
        ? process.env[key]
        : this.defaultValues[key];

      if (value === undefined) {
        compiler.hooks.thisCompilation.tap(
          'EnvironmentPlugin',
          (compilation) => {
            const error = new WebpackError(
              `EnvironmentPlugin - ${key} environment variable is undefined.\n\n` +
                'You can pass an object with default values to suppress this warning.\n' +
                'See https://rspack.rs/plugins/webpack/environment-plugin for example.',
            );

            error.name = 'EnvVariableNotDefinedError';
            compilation.errors.push(error);
          },
        );
      }

      // Expose each variable as both `process.env.KEY` and `import.meta.env.KEY`.
      // EnvironmentPlugin is just a thin DefinePlugin wrapper (matching webpack) —
      // the merging of these per-key defines into the whole `import.meta.env`
      // object is handled by ImportMetaPlugin. Env values are string data;
      // `JSON.stringify` turns them into code-string literals (DefinePlugin emits
      // string values verbatim as code fragments). `undefined` becomes the
      // `undefined` identifier. `DotenvPlugin` reuses this by passing its
      // resolved env as the default-values map.
      const defValue =
        value === undefined ? 'undefined' : JSON.stringify(value);
      definitions[`process.env.${key}`] = defValue;
      definitions[`import.meta.env.${key}`] = defValue;
    }
    new DefinePlugin(definitions).apply(compiler);
  }
}

export { EnvironmentPlugin };
