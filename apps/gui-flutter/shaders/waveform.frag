#version 320 es

// Spectral DJ waveform. One texel per bucket: R low, G mid, B high.
// One bar per pixel column, 0.46 x height scale, anti-aliased top/bottom
// edges, opaque lane fill so recorded strip pictures stay opaque.

precision highp float;

#include <flutter/runtime_effect.glsl>

uniform sampler2D uPeaks;

uniform vec2 uSize;        // picture size in px
uniform float uPeakCount;  // uPeaks width, >= 1
uniform float uMode;       // 0.0 RGB, 1.0 Filtered
uniform vec4 uBackground;  // lane fill (opaque)
uniform vec4 uLowColor;
uniform vec4 uMidColor;
uniform vec4 uHighColor;

out vec4 fragColor;

// Share of the row [center - 0.5, center + 0.5] that falls inside
// [mid - halfH, mid + halfH].
float coverage(float center, float mid, float halfH) {
  return clamp(
    min(center + 0.5, mid + halfH) - max(center - 0.5, mid - halfH),
    0.0,
    1.0
  );
}

void main() {
  vec2 p = FlutterFragCoord();
  float frac = clamp(p.x / uSize.x, 0.0, 1.0);
  float texel = (frac * (uPeakCount - 1.0) + 0.5) / uPeakCount;
  vec4 peak = texture(uPeaks, vec2(texel, 0.5));

  float midY = uSize.y * 0.5;
  float maxAmp = uSize.y * 0.46;
  vec3 outRgb = uBackground.rgb;

  if (uMode > 0.5) {
    // Mixxx "Filtered": low (widest) under mid under high, drawn in order.
    float lowH = peak.r * maxAmp;
    float midH = peak.g * maxAmp;
    float highH = peak.b * maxAmp;
    if (lowH > 0.1) {
      outRgb = mix(outRgb, uLowColor.rgb, coverage(p.y, midY, lowH));
    }
    if (midH > 0.1) {
      outRgb = mix(outRgb, uMidColor.rgb, coverage(p.y, midY, midH));
    }
    if (highH > 0.1) {
      outRgb = mix(outRgb, uHighColor.rgb, coverage(p.y, midY, highH));
    }
  } else {
    float amp = max(max(peak.r, peak.g), peak.b);
    float halfH = amp * maxAmp;
    if (amp > 0.001 && halfH > 0.1) {
      float total = peak.r + peak.g + peak.b + 1e-6;
      vec3 rgb =
        (peak.r * uLowColor.rgb +
         peak.g * uMidColor.rgb +
         peak.b * uHighColor.rgb) / total;
      float alpha = (0.65 + amp * 0.35) * coverage(p.y, midY, halfH);
      outRgb = mix(outRgb, rgb, alpha);
    }
  }

  fragColor = vec4(outRgb, 1.0);
}
