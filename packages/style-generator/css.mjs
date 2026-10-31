// @ts-check
// CSS emitter — `tokens.css`, the website's custom properties.
//
// Only colours live here: gradients, `color-mix()` mixes, the Tailwind
// `@theme inline` bridge and the `.band-teal` overrides stay authored in
// `apps/website/src/styles/global.css`, because DTCG cannot express them.

import { groupTokens } from './dictionary.mjs';
import { WEB } from './paths.mjs';

/** @typedef {import('style-dictionary/types').FormatFnArguments} FormatFnArguments */
/** @typedef {import('style-dictionary/types').TransformedToken} TransformedToken */

/**
 * @param {string} selector
 * @param {TransformedToken[]} tokens
 * @returns {string[]}
 */
const block = (selector, tokens) => [
  `${selector} {`,
  ...tokens.map((token) => {
    const comment = token.$description
      ? ` /* ${token.$description.replace(/\s+/g, ' ')} */`
      : '';
    return `  --${token.path[3]}: ${token.$value};${comment}`;
  }),
  '}',
];

/**
 * @type {(args: FormatFnArguments) => string}
 */
export const cssTokens = ({ dictionary }) =>
  [
    `/* GENERATED FILE — do not edit. Source: ${WEB}.`,
    ' * Regenerate with `npm run tokens:build`; `npm run tokens:check` fails CI',
    ' * when this file and the tokens disagree. */',
    '',
    ...block(
      ':root,\n[data-theme="light"]',
      groupTokens(dictionary, ['web', 'color', 'light']),
    ),
    '',
    ...block(
      '[data-theme="dark"]',
      groupTokens(dictionary, ['web', 'color', 'dark']),
    ),
    '',
  ].join('\n');
