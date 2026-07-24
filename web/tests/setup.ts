import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

afterEach(() => {
  cleanup();
  document.documentElement.classList.remove("dark");
  document.documentElement.removeAttribute("data-theme");
  document.documentElement.style.removeProperty("color-scheme");
});
