"use client";

import { Button } from "./ui/button";

export const ShareButton = () => (
  <Button
    variant="ghost"
    onClick={() => navigator.clipboard.writeText(window.location.toString())}
  >
    Share
  </Button>
);
