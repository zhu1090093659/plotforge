/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        ink: "#17202a",
        parchment: "#f7f3e8",
        signal: "#b23a48",
        brass: "#b98527",
        jade: "#2f7d69",
        graphite: {
          950: "#111419",
          900: "#171b21",
          850: "#1c222a",
          800: "#242b35",
          700: "#323b47",
        },
        canvas: {
          50: "#fffaf0",
          100: "#f7f0df",
          200: "#eadbbb",
        },
        amber: {
          400: "#e5b55f",
          500: "#c98b2f",
          600: "#9f6822",
        },
        health: {
          400: "#60b889",
          500: "#34815f",
        },
        acp: {
          400: "#65c6d3",
          500: "#2e8ca0",
        },
        agent: {
          400: "#9a82ce",
          500: "#7559a8",
        },
      },
      fontFamily: {
        sans: [
          "Aptos",
          "ui-sans-serif",
          "system-ui",
          "Apple Color Emoji",
          "Segoe UI Emoji",
        ],
      },
      boxShadow: {
        "studio-panel": "0 18px 48px rgba(17, 20, 25, 0.18)",
        "studio-dock": "0 -16px 40px rgba(17, 20, 25, 0.22)",
      },
    },
  },
  plugins: [],
};
