/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        // Warm ink — primary text on the parchment canvas. Tinted toward amber,
        // never the cool blue-black of generic AI dark themes.
        ink: "#2a2218",
        // Aged parchment — the warm cream of the writing-desk content surface.
        parchment: "#f1e8d4",
        // Forge signal red — danger / error accent. Warm, slightly desaturated.
        signal: "#b23a48",
        // Antique brass — a secondary warm metallic, used sparingly.
        brass: "#b98527",
        // Jade — health / success green, the workshop's patina accent.
        jade: "#2f7d69",
        // Graphite — the workshop shell chrome. Warm coal, tinted toward umber,
        // never the cold blue-gray of default dark UIs.
        graphite: {
          950: "#1f1a14",
          900: "#271f18",
          850: "#2e251e",
          800: "#382e25",
          700: "#4e4238",
        },
        // Canvas — the warm paper / content area. Tinted cream, not pure white.
        canvas: {
          50: "#f9f2e0",
          100: "#f4ead4",
          200: "#e6d9ba",
        },
        // Amber — the primary action accent. Workshop gold / lacquer.
        amber: {
          400: "#d6a050",
          500: "#c98b2f",
          600: "#9f6822",
        },
        // Health — success / proof-valid green.
        health: {
          400: "#5fa882",
          500: "#34815f",
        },
        // Accent — secondary highlight (surface nav, focus rings). Forge copper,
        // warm and distinct from amber gold. Replaces the prior cool cyan.
        accent: {
          400: "#c07440",
          500: "#a85c34",
        },
        // Agent — agent-mesh accent. Aged brass patina: a warm-tinted green
        // counterpoint, not the AI-slop cool purple it replaces.
        agent: {
          400: "#4a8a78",
          500: "#2d6258",
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
        // Warm-tinted shadows — never neutral gray. Echoes a desk lamp, not fog.
        "studio-panel": "0 18px 48px rgba(31, 26, 20, 0.16)",
        "studio-dock": "0 -14px 36px rgba(31, 26, 20, 0.20)",
        "shell-inset": "inset 0 1px 0 rgba(214, 160, 80, 0.06)",
      },
      letterSpacing: {
        tightish: "-0.018em",
        display: "-0.028em",
      },
    },
  },
  plugins: [],
};
