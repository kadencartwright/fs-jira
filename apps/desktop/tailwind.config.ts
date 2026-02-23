import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        canvas: "#070b12",
        panel: "#111827",
        ink: "#e6edf8",
        muted: "#9aa8bd",
        accent: "#14b8a6",
        warn: "#f59e0b",
        danger: "#ef4444",
      },
    },
  },
  plugins: [],
};

export default config;
