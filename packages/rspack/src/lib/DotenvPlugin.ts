/*
	MIT License http://www.opensource.org/licenses/mit-license.php
	Author Natsu @xiaoxiaojx
*/

import fs from 'node:fs';
import path from 'node:path';

import type { Compiler } from '../Compiler';
import { EnvironmentPlugin } from './EnvironmentPlugin';

type Prefix = string[];
type Env = Record<string, string>;

export interface DotenvPluginOptions {
  /**
   * The directory from which .env files are loaded. Can be an absolute path,
   * false will disable the .env file loading.
   */
  dir?: false | string;
  /**
   * Only expose environment variables that start with these prefixes.
   * Defaults to 'WEBPACK_'.
   */
  prefix?: string | string[];
  /**
   * Template patterns for .env file names. Use [mode] as placeholder for the
   * webpack mode.
   */
  template?: string[];
}

const DEFAULT_TEMPLATE = [
  '.env',
  '.env.local',
  '.env.[mode]',
  '.env.[mode].local',
];

const LINE =
  /^\s*(?:export\s+)?([\w.-]+)(?:\s*=\s*?|:\s+?)(\s*'(?:\\'|[^'])*'|\s*"(?:\\"|[^"])*"|\s*`(?:\\`|[^`])*`|[^#\r\n]+)?\s*(?:#.*)?$/gm;

const PLUGIN_NAME = 'DotenvPlugin';

function parse(src: string | Buffer): Env {
  const obj = Object.create(null) as Env;

  let lines = src.toString();
  lines = lines.replace(/\r\n?/g, '\n');

  let match: null | RegExpExecArray;
  while ((match = LINE.exec(lines)) !== null) {
    const key = match[1];

    let value = match[2] || '';
    value = value.trim();

    const maybeQuote = value[0];
    value = value.replace(/^(['"`])([\s\S]*)\1$/gm, '$2');

    if (maybeQuote === '"') {
      value = value.replace(/\\n/g, '\n');
      value = value.replace(/\\r/g, '\r');
    }

    obj[key] = value;
  }

  return obj;
}

function resolveEscapeSequences(value: string): string {
  return value.replace(/\\\$/g, '$');
}

function expandValue(
  value: string,
  processEnv: Record<string, string | undefined>,
  runningParsed: Env,
): string {
  const env = { ...runningParsed, ...processEnv };
  const regex = /(?<!\\)\$\{([^{}]+)\}|(?<!\\)\$([a-z_]\w*)/gi;

  let result = value;
  let match: null | RegExpExecArray;
  const seen = new Set<string>();

  while ((match = regex.exec(result)) !== null) {
    seen.add(result);

    const [template, bracedExpression, unbracedExpression] = match;
    const expression = bracedExpression || unbracedExpression;
    const opRegex = /(:\+|\+|:-|-)/;
    const opMatch = expression.match(opRegex);
    const splitter = opMatch ? opMatch[0] : null;
    const r = expression.split(splitter as string);
    let defaultValue: string;
    let value: undefined | null | string;
    const key = r.shift();

    if ([':+', '+'].includes(splitter || '')) {
      defaultValue = env[key || ''] ? r.join(splitter || '') : '';
      value = null;
    } else {
      defaultValue = r.join(splitter || '');
      value = env[key || ''];
    }

    if (value) {
      result = seen.has(value)
        ? result.replace(template, defaultValue)
        : result.replace(template, value);
    } else {
      result = result.replace(template, defaultValue);
    }

    if (result === runningParsed[key || '']) {
      break;
    }

    regex.lastIndex = 0;
  }

  return result;
}

function expand(options: {
  parsed: Env;
  processEnv: Record<string, string | undefined>;
}): { parsed: Env } {
  const runningParsed = Object.create(null) as Env;
  const processEnv = options.processEnv;

  for (const key in options.parsed) {
    let value = options.parsed[key];
    value =
      Object.prototype.hasOwnProperty.call(processEnv, key) &&
      processEnv[key] !== value
        ? (processEnv[key] as string)
        : expandValue(value, processEnv, runningParsed);

    const resolvedValue = resolveEscapeSequences(value);

    options.parsed[key] = resolvedValue;
    runningParsed[key] = resolvedValue;
  }

  return options;
}

class DotenvPlugin {
  options: DotenvPluginOptions;

  constructor(options: DotenvPluginOptions = {}) {
    this.options = options;
  }

  apply(compiler: Compiler) {
    const dir =
      this.options.dir === false ? false : this.options.dir || compiler.context;
    const prefixes =
      typeof this.options.prefix === 'string'
        ? [this.options.prefix]
        : this.options.prefix || ['WEBPACK_'];

    const { parsed, fileDependencies, missingDependencies } = dir
      ? this.getParsed(dir, compiler.options.mode || 'development')
      : {
          parsed: Object.create(null) as Env,
          fileDependencies: [] as string[],
          missingDependencies: [] as string[],
        };
    const env = this.getEnv(prefixes, parsed);

    // Expose each resolved variable as both `process.env.KEY` and
    // `import.meta.env.KEY` by delegating to `EnvironmentPlugin`, which is the
    // shared per-key DefinePlugin wrapper for env variables. `env` is passed as
    // the default-values map: `EnvironmentPlugin` still prefers an actual
    // `process.env[key]` when present (matching `getEnv`'s "process.env wins"
    // rule) and otherwise falls back to the resolved (expanded) value. The
    // merging of these per-key defines into the whole `import.meta.env` object
    // is handled by ImportMetaPlugin, so no compiler-internal env accumulator is
    // touched here.
    new EnvironmentPlugin(env).apply(compiler);

    compiler.hooks.compilation.tap(PLUGIN_NAME, (compilation) => {
      compilation.fileDependencies.addAll(fileDependencies);
      compilation.missingDependencies.addAll(missingDependencies);
    });
  }

  getEnvFilesForMode(dir: string, mode: string | undefined): string[] {
    const templates = this.options.template || DEFAULT_TEMPLATE;

    return templates
      .map((pattern) => pattern.replace(/\[mode\]/g, mode || 'development'))
      .map((file) => path.join(dir, file));
  }

  getParsed(
    dir: string,
    mode: string,
  ): {
    parsed: Env;
    fileDependencies: string[];
    missingDependencies: string[];
  } {
    const fileDependencies: string[] = [];
    const missingDependencies: string[] = [];
    const envFiles = this.getEnvFilesForMode(dir, mode);
    const parsed = Object.create(null) as Env;

    for (const filePath of envFiles) {
      let content = '';
      try {
        content = fs.readFileSync(filePath, 'utf-8');
        fileDependencies.push(filePath);
      } catch {
        missingDependencies.push(filePath);
      }

      if (!content) continue;
      const entries = parse(content);
      for (const key in entries) {
        parsed[key] = entries[key];
      }
    }

    return { parsed, fileDependencies, missingDependencies };
  }

  getEnv(prefixes: Prefix, parsed: Env): Env {
    const processEnv = { ...process.env };
    expand({ parsed, processEnv });
    const env = Object.create(null) as Env;
    const keys = [...Object.keys(parsed), ...Object.keys(process.env)];

    for (const key of keys) {
      if (prefixes.some((prefix) => key.startsWith(prefix))) {
        env[key] =
          Object.prototype.hasOwnProperty.call(process.env, key) &&
          process.env[key]
            ? process.env[key]
            : parsed[key];
      }
    }

    return env;
  }
}

export { DotenvPlugin };
