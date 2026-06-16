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
    const definitions = compiler.__internal__get_environment();
    for (const key of this.keys) {
      const value =
        process.env[key] !== undefined
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

      // Env values are string data; convert them to code-string literals
      // (matching webpack's EnvironmentPlugin) so DefinePlugin emits them as
      // string literals. `undefined` becomes the `undefined` identifier.
      definitions[key] =
        value === undefined ? 'undefined' : JSON.stringify(value);
    }
  }
}

export { EnvironmentPlugin };
