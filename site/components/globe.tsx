"use client";

import createGlobe from "cobe";
import { useTheme } from "next-themes";
import { useEffect, useRef } from "react";

export const Globe = () => {
  const canvasRef = useRef();
  const theme = useTheme();

  useEffect(() => {
    let phi = 4;

    const globe = createGlobe(canvasRef.current, {
      devicePixelRatio: 2,
      width: 600 * 2,
      height: 600 * 2,
      phi,
      theta: 0.2,
      dark: theme.theme === "dark" ? 0.99 : 0.0,
      diffuse: 0.7,
      mapSamples: 25000,
      mapBrightness: 1.4,
      baseColor: theme.theme === "dark" ? [0.15, 0.8, 0.4] : [0.8, 1, 0.9],
      markerColor: theme.theme === "dark" ? [0.1, 0.8, 0.2] : [0.3, 1, 0.6],
      glowColor: theme.theme === "dark" ? [0.1, 0.7, 0.3] : [0.7, 1, 0.8],
      markers: [],
      onRender: (state) => {
        // Called on every animation frame.
        // `state` will be an empty object, return updated params.
        state.phi = phi;
        phi += 0.001;
      },
    });

    return () => {
      globe.destroy();
    };
  }, [theme.theme, canvasRef.current]);

  return (
    <canvas
      ref={canvasRef}
      style={{ width: 600, height: 600, maxWidth: "100%", aspectRatio: 1 }}
    />
  );
};
