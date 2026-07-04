/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        // Ink — primary text on the lavender-white canvas. Deep aubergine,
        // tinted toward violet (never the cold blue-black of generic dark
        // themes nor the warm umber of the prior forge palette). Deepened
        // for stronger display hierarchy against the illuminated canvas.
        ink: "#2b2238",
        // Lavender white — the default canvas. A whisper of violet, never
        // cold blue-white nor the prior warm cream.
        parchment: "#faf8ff",
        // Signal — danger / error accent. A muted berry-red that lives in the
        // violet family, not a fire-engine red.
        signal: "#a8426b",
        // Plum — secondary accent for status / success chips. A desaturated
        // grape, warmer than violet but still in-family. Replaces prior brass.
        plum: {
          400: "#9a7ab0",
          500: "#7a5a8a",
        },
        // Sage — success / proof-valid green. A muted, cool sage that
        // complements violet without fighting it. Replaces prior jade.
        sage: "#5a8a7a",
        // Copper — the warm metallic counterpoint to violet. Used sparingly
        // for numbered eyebrows, hairline accents, selected-card rules, and
        // the "agent" side of two-tone compositions. Aged-penny, not bright
        // orange, so it stays inside the candlelit-manuscript family.
        copper: {
          400: "#b58a6e",
          500: "#8a6450",
          600: "#6b4d3d",
        },
        // Paper — the lavender-white chrome surfaces (sidebar, evidence panel,
        // command dock). Tinted toward lilac, never cold blue-gray.
        paper: {
          50: "#fdfbff",
          100: "#faf8ff",
          200: "#f3f0fb",
          300: "#ebe6f5",
          400: "#ded7ee",
          500: "#cdc3e2",
        },
        // Canvas — the lavender-white content area.
        canvas: {
          50: "#fdfbff",
          100: "#f5f2fc",
          200: "#ded7ee",
        },
        // Violet — the primary action accent. An editorial, slightly muted
        // violet (not the saturated AI-slop neon purple). Pairs restraint with
        // a literary, manuscript-by-candlelight feel. Replaces prior amber.
        violet: {
          300: "#a893cc",
          400: "#8a6fb8",
          500: "#6f54a3",
          600: "#594085",
          700: "#473366",
        },
        // Health — success / proof-valid green (kept as a token name for the
        // chip tone system; mirrors sage).
        health: {
          400: "#6a9a8a",
          500: "#5a8a7a",
        },
        // Accent — secondary highlight (surface nav, focus rings). A softer
        // lilac-copper, warm-cool bridged so focus rings feel intentional
        // against the violet palette.
        accent: {
          400: "#9a7ab5",
          500: "#7a5a95",
        },
        // Agent — agent-mesh accent. A deep iris that reads as a violet-family
        // counterpoint, not the AI-slop cool purple-to-blue gradient.
        agent: {
          400: "#6a5a9a",
          500: "#4d3e7a",
        },
        // Muted — for secondary/muted text on light surfaces. Tinted toward
        // violet-gray, never warm brown.
        muted: {
          700: "#6b6378",
          800: "#4a4458",
        },
        // Graphite — legacy scale retained as a light-theme alias so existing
        // chrome surfaces (bg-graphite-*) render as lavender paper and muted
        // text (text-graphite-700) reads as violet-gray. New code should
        // prefer the `paper` / `muted` scales; this alias avoids a brittle
        // same-PR rename of every chrome token across the views.
        graphite: {
          950: "#faf8ff",
          900: "#f3f0fb",
          850: "#ebe6f5",
          800: "#ded7ee",
          700: "#6b6378",
        },
      },
      fontFamily: {
        // Fraunces — a modern serif with optical sizing & a "soft" wedge to it;
        // used for display headings. Pairs warmth with literary precision.
        display: [
          "Fraunces",
          "ui-serif",
          "Georgia",
          "Cambria",
          "Times New Roman",
          "serif",
        ],
        // Atkinson Hyperlegible — a humanist sans engineered for legibility;
        // used for body, UI chrome, and labels. Distinctive without being trendy.
        sans: [
          "Atkinson Hyperlegible",
          "ui-sans-serif",
          "system-ui",
          "-apple-system",
          "BlinkMacSystemFont",
          "Segoe UI",
          "sans-serif",
        ],
        // Mono — for source / trace code surfaces. Kept utilitarian.
        mono: [
          "ui-monospace",
          "SFMono-Regular",
          "Menlo",
          "Consolas",
          "monospace",
        ],
      },
      boxShadow: {
        // Violet-tinted shadows — a soft lilac haze, never neutral gray nor
        // the prior warm-umber desk-lamp shadow.
        "studio-panel": "0 18px 48px rgba(90, 70, 130, 0.10)",
        // Lift — a tighter, warmer shadow for hover-raised cards. Two-layer
        // (contact + ambient) so cards feel like they rise off the parchment
        // rather than float in a generic drop-shadow slop.
        "panel-lift":
          "0 1px 2px rgba(43, 34, 56, 0.04), 0 24px 64px rgba(90, 70, 130, 0.12)",
        "studio-dock": "0 -10px 28px rgba(90, 70, 130, 0.10)",
        "shell-inset": "inset 0 1px 0 rgba(138, 111, 184, 0.20)",
      },
      transitionTimingFunction: {
        // Exponential ease-out (ease-out-quint-ish). Real objects decelerate
        // smoothly; never bounce/elastic.
        expo: "cubic-bezier(0.16, 1, 0.3, 1)",
      },
      letterSpacing: {
        tightish: "-0.018em",
        display: "-0.028em",
        "display-tight": "-0.034em",
        // Eyebrow — wide, uppercase micro-labels above display headings.
        eyebrow: "0.18em",
      },
      keyframes: {
        // Page-load reveal — opacity + small translateY, expo deceleration.
        "reveal-in": {
          "0%": { opacity: "0", transform: "translateY(6px)" },
          "100%": { opacity: "1", transform: "translateY(0)" },
        },
        // Scene image slow scale-in (opacity only, no Ken-Burns drift).
        "scene-fade-in": {
          "0%": { opacity: "0" },
          "100%": { opacity: "1" },
        },
      },
      animation: {
        "reveal-in": "reveal-in 0.5s cubic-bezier(0.16, 1, 0.3, 1) both",
        "scene-fade-in": "scene-fade-in 0.6s cubic-bezier(0.16, 1, 0.3, 1) both",
      },
    },
  },
  plugins: [],
};
