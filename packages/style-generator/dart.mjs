// @ts-check
// Dart emitters — `mixar_palette.dart` (primitive ladders) and
// `mixar_colors.dart` (semantic tokens).
//
// Primitive references stay symbolic (`MixarBone.s100`) by reading each token's
// *original* `$value`, so a ladder step is authored once and every semantic
// token that names it stays readable in the generated file.

import { readFileSync } from 'node:fs';
import { groupTokens } from './dictionary.mjs';
import { at, PRIMITIVES, SEMANTIC } from './paths.mjs';

/** @typedef {import('style-dictionary/types').FormatFnArguments} FormatFnArguments */
/** @typedef {import('style-dictionary/types').TransformedToken} TransformedToken */
/** @typedef {import('style-dictionary/types').TransformedTokens} TransformedTokens */

/** Group descriptions are dropped by the resolver, so read the authors' copy. */
const primitiveDocs = /** @type {Record<string, string | undefined>} */ (
  Object.fromEntries(
    Object.entries(
      /** @type {Record<string, {$description?: string}>} */ (
        JSON.parse(
          readFileSync(
            new URL('../../tokens/primitives.json', import.meta.url),
            'utf8',
          ),
        ).color ?? {}
      ),
    ).map(([name, group]) => [name, group.$description]),
  )
);

/* ------------------------------------------------------------------ helpers */

/** PascalCase Dart class for a primitive ladder: `bone` → `MixarBone`. */
/** @type {(name: string) => string} */
const ladderClass = (name) => `Mixar${name[0].toUpperCase()}${name.slice(1)}`;

/**
 * The authored `$value` of a token, before Style Dictionary resolves aliases.
 * v4/v5 have shipped both `$value` and `value`, so read either.
 *
 * @param {TransformedToken} token
 * @returns {unknown}
 */
const originalValue = (token) =>
  token.original?.$value ?? token.original?.value ?? token.$value;

/**
 * `#rrggbb` / `#rrggbbaa` → `Color(0xAARRGGBB)`.
 *
 * @param {unknown} raw
 * @returns {string}
 */
const dartHex = (raw) => {
  const match = /^#([0-9a-fA-F]{6})([0-9a-fA-F]{2})?$/.exec(String(raw).trim());
  if (!match) throw new Error(`not a hex colour: ${String(raw)}`);
  const alpha = (match[2] ?? 'FF').toUpperCase();
  return `Color(0x${alpha}${match[1].toUpperCase()})`;
};

/**
 * A Dart expression for a colour token. References stay symbolic so the
 * generated file keeps naming the ladder step it came from; literals — the few
 * colours that belong to no ladder — become `Color(0x…)`.
 *
 * @param {TransformedToken} token
 * @returns {string}
 */
const dartValue = (token) => {
  const authored = originalValue(token);
  if (typeof authored === 'string' && authored.startsWith('{')) {
    const parts = authored.slice(1, -1).split('.');
    if (parts[0] !== 'color' || parts.length !== 3) {
      throw new Error(`unmapped token reference: ${authored}`);
    }
    return `${ladderClass(parts[1])}.s${parts[2]}`;
  }
  return dartHex(authored);
};

/**
 * Wrap a token description into `///` lines at the 80-column limit.
 *
 * @param {string | undefined} text
 * @param {string} [indent]
 * @returns {string[]}
 */
const dartDoc = (text, indent = '') => {
  if (!text) return [];
  const limit = 80 - indent.length - 4;
  const words = text.trim().replace(/\s+/g, ' ').split(' ');
  /** @type {string[]} */
  const lines = [];
  let line = '';
  for (const word of words) {
    if (line && line.length + 1 + word.length > limit) {
      lines.push(line);
      line = word;
    } else {
      line = line ? `${line} ${word}` : word;
    }
  }
  if (line) lines.push(line);
  return lines.map((l) => `${indent}/// ${l}`);
};

/** @type {(sources: string) => string} */
const generatedHeader = (sources) =>
  [
    '// GENERATED FILE — do not edit.',
    '//',
    `// Source: ${sources}. Regenerate with \`npm run tokens:build\`;`,
    '// `npm run tokens:check` fails CI when this file and the tokens disagree.',
  ].join('\n');

/* ------------------------------------------------------------------ formats */

/**
 * Barrel of primitive ladders — what `mixar_palette.dart` used to hold.
 *
 * @type {(args: FormatFnArguments) => string}
 */
export const dartPalette = ({ dictionary }) => {
  const ladders = Object.entries(
    /** @type {TransformedTokens} */ (dictionary.tokens.color),
  ).filter(([name]) => name !== '$type' && name !== '$description');

  const blocks = ladders.map(([name, ladder]) => {
    const steps = Object.entries(
      /** @type {TransformedTokens} */ (ladder),
    ).filter(([key]) => key !== '$description');
    const body = steps.map(
      ([step, token]) =>
        `  static const s${step} = ${dartHex(/** @type {TransformedToken} */ (token).$value)};`,
    );
    const title = `${ladderClass(name).slice('Mixar'.length)} scale`;
    const description = primitiveDocs[name];
    return [
      '',
      ...(description
        ? dartDoc(`${title} — ${description}`)
        : dartDoc(`${title}.`)),
      `abstract final class ${ladderClass(name)} {`,
      ...body,
      '}',
    ].join('\n');
  });

  return [
    generatedHeader(PRIMITIVES),
    '',
    "import 'package:flutter/painting.dart';",
    '',
    '// Solid Mixar colour scales (Tailwind-aligned).',
    '//',
    '// Prefer these over `withValues(alpha: …)` / `Color.alphaBlend` for fills,',
    '// borders, and icons. Alpha stays for true see-through surfaces only',
    '// (dialogs, dimming scrims) that must show content behind them.',
    '//',
    '// Brand signal is teal. Deck identity is sky (A) / rose (B) — never a third',
    '// green/emerald ladder.',
    ...blocks,
    '',
  ].join('\n');
};

/**
 * `mixar_colors.dart` — the semantic tokens plus the behaviour that reads them.
 *
 * The three methods are stable algorithms over whatever tokens exist, so they
 * ride along in the template rather than in a half-generated file.
 *
 * @type {(args: FormatFnArguments) => string}
 */
export const dartColors = ({ dictionary }) => {
  const light = groupTokens(dictionary, ['semantic', 'color', 'light']);
  const dark = groupTokens(dictionary, ['semantic', 'color', 'dark']);
  const lightNames = light.map((token) => token.path[3]);
  const darkNames = dark.map((token) => token.path[3]);
  if (
    lightNames.length !== darkNames.length ||
    lightNames.some((name, i) => name !== darkNames[i])
  ) {
    throw new Error(
      'tokens/semantic.json: light and dark must declare the same tokens, in the same order',
    );
  }

  const names = lightNames;
  /** @type {Map<string, string>} */
  const docs = new Map(
    light
      .filter((token) => token.$description)
      .map((token) => [
        token.path[3],
        /** @type {string} */ (token.$description),
      ]),
  );

  /**
   * @param {string} theme
   * @param {TransformedToken[]} tokens
   * @returns {string[]}
   */
  const instance = (theme, tokens) => [
    `  static const ${theme} = MixarColors(`,
    `    brightness: Brightness.${theme},`,
    ...tokens.map((token) => `    ${token.path[3]}: ${dartValue(token)},`),
    '  );',
  ];

  return [
    generatedHeader(SEMANTIC),
    '',
    "import 'dart:ui';",
    '',
    "import 'package:flutter/foundation.dart';",
    "import 'package:flutter/painting.dart';",
    "import 'package:gui_flutter/core/theme/mixar_accent.dart';",
    "import 'package:gui_flutter/core/theme/mixar_palette.dart';",
    '',
    '/// Mixar color tokens aligned with the website palette.',
    '@immutable',
    'class MixarColors {',
    '  const MixarColors({',
    '    required this.brightness,',
    ...names.map((name) => `    required this.${name},`),
    '    this.hoverLighten = 0.075,',
    '    this.hoverDarken = 0.05,',
    '    this.disabledOpacity = 0.5,',
    '  });',
    '',
    '  /// Desktop light — warm bone paper, mirrored by website `global.css` light',
    '  /// theme. Surfaces climb the bone ladder (`background` s100, `card` s50), so',
    '  /// no panel is pure white and raised planes read apart from the page.',
    ...instance('light', light),
    '',
    '  /// Desktop dark — booth chrome, mirrored by the website dark theme.',
    ...instance('dark', dark),
    '',
    '  final Brightness brightness;',
    ...names.flatMap((name) => [
      ...(docs.has(name) ? ['', ...dartDoc(docs.get(name), '  ')] : []),
      `  final Color ${name};`,
    ]),
    '  final double hoverLighten;',
    '  final double hoverDarken;',
    '  final double disabledOpacity;',
    '',
    '  /// Accent-seeded copy: same neutrals, cue amber and hairlines, brand tokens',
    '  /// re-read from [scale]. Shade slots are fixed per brightness, so every',
    '  /// palette keeps the contrast the teal default was tuned for.',
    '  MixarColors withAccentScale(MixarColorScale scale) {',
    '    final dark = brightness == Brightness.dark;',
    '    return MixarColors(',
    '      brightness: brightness,',
    '      background: background,',
    '      foreground: foreground,',
    '      primary: dark ? scale.s400 : scale.s700,',
    '      primaryForeground: primaryForeground,',
    '      secondary: secondary,',
    '      secondaryForeground: secondaryForeground,',
    '      muted: muted,',
    '      mutedForeground: mutedForeground,',
    '      accent: dark ? scale.s400 : scale.s700,',
    '      primaryHover: dark ? scale.s300 : scale.s800,',
    '      destructive: destructive,',
    '      destructiveForeground: destructiveForeground,',
    '      card: card,',
    '      waveformBackground: waveformBackground,',
    '      border: border,',
    '      selection: dark ? scale.s950 : scale.s100,',
    '      selectionForeground: selectionForeground,',
    '      primaryTint: dark ? scale.s950 : scale.s50,',
    '      primaryDim: dark ? scale.s600 : scale.s500,',
    '      accentTint: dark ? scale.s950 : scale.s50,',
    '      cueTint: cueTint,',
    '      cueOn: cueOn,',
    '      hoverLighten: hoverLighten,',
    '      hoverDarken: hoverDarken,',
    '      disabledOpacity: disabledOpacity,',
    '    );',
    '  }',
    '',
    '  /// Hovered variant (Forui `FColors.hover` algorithm).',
    '  Color hover(Color color) {',
    '    final hsl = HSLColor.fromColor(color);',
    '    final l = hsl.lightness;',
    '    final (space, factor, sign) = l > 0.5',
    '        ? (1.0 - l, hoverDarken, -1)',
    '        : (l, hoverLighten, 1);',
    '    final aggressiveness = 1 + ((0.5 - space) / 0.5);',
    '    final adjustment = factor * aggressiveness * sign;',
    '    final lightness = clampDouble(l + adjustment, 0, 1);',
    '    final hovered = hsl.withLightness(lightness).toColor();',
    '    if (hovered.colorSpace != color.colorSpace) {',
    '      return hovered.withValues(colorSpace: color.colorSpace);',
    '    }',
    '    return hovered;',
    '  }',
    '',
    '  /// Disabled variant (Forui `FColors.disable` algorithm).',
    '  Color disable(Color color, [Color? background]) {',
    '    final disabled = color.withValues(alpha: color.a * disabledOpacity);',
    '    return background == null',
    '        ? disabled',
    '        : Color.alphaBlend(disabled, background);',
    '  }',
    '}',
    '',
  ].join('\n');
};
