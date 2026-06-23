/**
 * Verifies that `pathData.chunk` handed to a `filename` / `chunkFilename`
 * function is a real `Chunk` instance (with methods like `groupsIterable` and
 * `getEntryOptions`), not a plain `{ name, id, hash }` snapshot. The entry chunk
 * is distinguished from non-entry chunks by walking its chunk groups.
 *
 * @param {import("@rspack/core").PathData} pathData path data
 * @returns {string} the chunk filename
 */
function pickName(pathData) {
  const chunk = pathData.chunk;
  // A plain object fallback would not have these methods.
  if (
    typeof chunk.getEntryOptions !== 'function' ||
    typeof chunk.groupsIterable[Symbol.iterator] !== 'function'
  ) {
    throw new Error('pathData.chunk is not a real Chunk instance');
  }
  const isEntryChunk = [...chunk.groupsIterable].some(
    (group) => group.isInitial() && group.getEntrypointChunk() === chunk,
  );
  return isEntryChunk ? 'entry-[name].js' : 'async-[name].js';
}

/** @type {import("@rspack/core").Configuration} */
module.exports = {
  entry: './index.js',
  output: {
    filename: pickName,
    chunkFilename: pickName,
  },
};
