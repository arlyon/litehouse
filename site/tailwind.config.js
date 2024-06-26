import animations from "@midudev/tailwind-animations";
import { createPreset } from "fumadocs-ui/tailwind-plugin";
import bgPatterns from "tailwindcss-bg-patterns";
import animate from "tailwindcss-animate";

/** @type {import('tailwindcss').Config} */
export default {
  dark: "class",
  content: [
    "./components/**/*.{ts,tsx}",
    "./app/**/*.{ts,tsx}",
    "./content/**/*.{md,mdx}",
    "./mdx-components.{ts,tsx}",
    "../node_modules/fumadocs-ui/dist/**/*.js",
  ],
  theme: {
    // defaults to these values
    patterns: {
      opacities: {
        100: "1",
        80: ".80",
        60: ".60",
        40: ".40",
        20: ".20",
        10: ".10",
        5: ".05",
      },
      sizes: {
        1: "0.25rem",
        2: "0.5rem",
        4: "1rem",
        6: "1.5rem",
        8: "2rem",
        16: "4rem",
        20: "5rem",
        24: "6rem",
        32: "8rem",
      },
    },
    extend: {
      keyframes: {
        "caret-blink": {
          "0%,70%,100%": { opacity: "1" },
          "20%,50%": { opacity: "0" },
        },
      },
      animation: {
        "caret-blink": "caret-blink 1.25s ease-out infinite",
      },
    },
  },
  plugins: [bgPatterns, animate],
  presets: [createPreset()],
  // plugins: [animations],
};
