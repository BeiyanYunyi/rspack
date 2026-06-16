import { BuiltinPluginName } from '@rspack/binding';

import { create } from './base';
import { Compiler } from '../Compiler';

const PROCESS_ENV = 'process.env' as const;
const IMPORT_META_ENV = 'import.meta.env' as const;

const PROCESS_ENV_PREFIX = `${PROCESS_ENV}.` as const;
const IMPORT_META_ENV_PREFIX = `${IMPORT_META_ENV}.` as const;

export type DefinePluginOptions = Record<string, CodeValue>;
export const DefinePlugin = create(
  BuiltinPluginName.DefinePlugin,
  function (define: DefinePluginOptions): NormalizedCodeValue {
    const supportsBigIntLiteral =
      this.options.output.environment?.bigIntLiteral ?? false;

    collectEnvDefinitions(this, define);

    return normalizeValue(define, supportsBigIntLiteral);
  },
  'compilation',
);

const collectEnvDefinitions = (
  compiler: Compiler,
  define: DefinePluginOptions,
): DefinePluginOptions => {
  const definitions = compiler.__internal__get_environment();

  function collectEnv(prefix: string, key: string, value: CodeValue) {
    const rawKey = key.slice(prefix.length);
    definitions[rawKey] = value;
  }

  for (const [key, value] of Object.entries(define)) {
    if (key === PROCESS_ENV || key === IMPORT_META_ENV) {
      Object.assign(definitions, value);
    } else if (key.startsWith(PROCESS_ENV_PREFIX)) {
      collectEnv(PROCESS_ENV_PREFIX, key, value);
    } else if (key.startsWith(IMPORT_META_ENV_PREFIX)) {
      collectEnv(IMPORT_META_ENV_PREFIX, key, value);
    }
  }

  return definitions;
};

const normalizeValue = (
  define: DefinePluginOptions,
  supportsBigIntLiteral: boolean,
) => {
  const normalizePrimitive = (
    p: CodeValuePrimitive,
  ): NormalizedCodeValuePrimitive => {
    if (p === undefined) {
      return 'undefined';
    }
    if (Object.is(p, -0)) {
      return '-0';
    }
    if (p instanceof RegExp) {
      return p.toString();
    }
    if (typeof p === 'function') {
      return `(${p.toString()})`;
    }
    if (typeof p === 'bigint') {
      return supportsBigIntLiteral ? `${p}n` : `BigInt("${p}")`;
    }
    // assume `p` is a valid JSON value
    return p;
  };
  const normalizeObject = (define: CodeValue): NormalizedCodeValue => {
    if (Array.isArray(define)) {
      return define.map(normalizeObject);
    }
    if (define instanceof RegExp) {
      return normalizePrimitive(define);
    }
    if (define && typeof define === 'object') {
      const keys = Object.keys(define);
      return Object.fromEntries(
        keys.map((k) => [k, normalizeObject(define[k])]),
      );
    }
    return normalizePrimitive(define);
  };
  return normalizeObject(define);
};

type CodeValue = RecursiveArrayOrRecord<CodeValuePrimitive>;
type CodeValuePrimitive =
  | null
  | undefined
  | RegExp
  | Function
  | string
  | number
  | boolean
  | bigint;
type NormalizedCodeValuePrimitive = null | string | number | boolean;
type NormalizedCodeValue = RecursiveArrayOrRecord<NormalizedCodeValuePrimitive>;

type RecursiveArrayOrRecord<T> =
  | { [index: string]: RecursiveArrayOrRecord<T> }
  | RecursiveArrayOrRecord<T>[]
  | T;
