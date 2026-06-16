import type { Compiler } from '../Compiler';
import type { DefinePluginOptions } from '../builtin-plugin';

export const registerEnvDefinitions = (
  compiler: Compiler,
  definitions: DefinePluginOptions,
): void => {
  const env = compiler.__internal__get_environment();
  for (const [key, value] of Object.entries(definitions)) {
    env[key] = value;
  }
};
