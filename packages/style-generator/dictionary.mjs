// @ts-check
// Queries over Style Dictionary's resolved token tree.

/** @typedef {import('style-dictionary/types').Dictionary} Dictionary */
/** @typedef {import('style-dictionary/types').TransformedToken} TransformedToken */

/**
 * Tokens under one group, in authored order: `['semantic','color','light']`.
 *
 * @param {Dictionary} dictionary
 * @param {string[]} path
 * @returns {TransformedToken[]}
 */
export const groupTokens = (dictionary, path) =>
  dictionary.allTokens.filter(
    (token) =>
      token.path.length > path.length &&
      path.every((segment, i) => token.path[i] === segment),
  );
