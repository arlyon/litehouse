"use client";

import { cva } from "class-variance-authority";
import { MoonIcon, SunIcon } from "lucide-react";
import { useTheme } from "next-themes";
import { type ButtonHTMLAttributes, useCallback } from "react";
import { twMerge as cn } from "tailwind-merge";

const buttonVariants = cva("size-7 rounded-full p-1.5 text-muted-foreground", {
  variants: {
    dark: {
      true: "dark:bg-accent dark:text-accent-foreground",
      false:
        "bg-accent text-accent-foreground dark:bg-transparent dark:text-muted-foreground",
    },
  },
});

export function ThemeToggle({
  className,
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement>): React.ReactElement {
  const { setTheme, resolvedTheme } = useTheme();

  const onToggle = useCallback(() => {
    setTheme(resolvedTheme === "dark" ? "light" : "dark");
  }, [setTheme, resolvedTheme]);

  return (
    <button
      type="button"
      className={cn(
        "inline-flex items-center rounded-full border p-0.5",
        className,
      )}
      aria-label="Toggle Theme"
      onClick={onToggle}
      {...props}
    >
      <SunIcon className={cn(buttonVariants({ dark: false }))} />
      <MoonIcon className={cn(buttonVariants({ dark: true }))} />
    </button>
  );
}
