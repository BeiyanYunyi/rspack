'use strict';

const { EnvironmentPlugin } = require('@rspack/core');

/** @type {import("@rspack/core").Configuration} */
module.exports = {
  target: 'node',
  plugins: [
    new EnvironmentPlugin({
      AAA: 'aaa',
    }),
  ],
  experiments: {
    outputModule: true,
  },
  output: {
    module: true,
    chunkFormat: 'module',
  },
  externals: {
    fs: 'commonjs fs',
  },
};
