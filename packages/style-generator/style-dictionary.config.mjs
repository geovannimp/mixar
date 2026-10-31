// @ts-check
// Style Dictionary config — the one authored source of Mixar's colours.
//
// `tokens/*.json` are DTCG documents. The emitters (`dart.mjs`, `css.mjs`) live
// next to this file; together they produce three generated outputs:
//
//   apps/gui-flutter/lib/core/theme/mixar_palette.dart  (tokens/primitives.json)
//   apps/gui-flutter/lib/core/theme/mixar_colors.dart   (tokens/semantic.json)
//   apps/website/src/styles/tokens.css                  (tokens/web.json)
//
// Paths are resolved from this file, not the working directory, so the task
// behaves the same whether moon runs it from the package or the repo root.
// `npm run tokens:build` regenerates; `npm run tokens:check` also fails when the
// committed files and the tokens disagree.

import StyleDictionary from 'style-dictionary';
import { cssTokens } from './css.mjs';
import { dartColors, dartPalette } from './dart.mjs';
import { at, PRIMITIVES, SEMANTIC, WEB } from './paths.mjs';

/** @typedef {import('style-dictionary/types').Config} Config */

StyleDictionary.registerFormat({
  name: 'dart/mixar-palette',
  format: dartPalette,
});
StyleDictionary.registerFormat({
  name: 'dart/mixar-colors',
  format: dartColors,
});
StyleDictionary.registerFormat({
  name: 'css/mixar-tokens',
  format: cssTokens,
});

/** @type {Config} */
const config = {
  source: [at(PRIMITIVES), at(SEMANTIC), at(WEB)],
  log: { warnings: 'warn', verbosity: 'default' },
  platforms: {
    flutter: {
      transformGroup: 'js',
      buildPath: at('apps/gui-flutter/lib/core/theme/'),
      files: [
        { destination: 'mixar_palette.dart', format: 'dart/mixar-palette' },
        { destination: 'mixar_colors.dart', format: 'dart/mixar-colors' },
      ],
    },
    website: {
      transformGroup: 'js',
      buildPath: at('apps/website/src/styles/'),
      files: [{ destination: 'tokens.css', format: 'css/mixar-tokens' }],
    },
  },
};

export default config;
