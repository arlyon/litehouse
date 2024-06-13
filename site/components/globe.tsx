"use client";

import createGlobe from "cobe";
import { useEffect, useRef } from "react";

export const Globe = () => {
  const canvasRef = useRef();

  useEffect(() => {
    let phi = 4;

    const globe = createGlobe(canvasRef.current, {
      devicePixelRatio: 2,
      width: 600 * 2,
      height: 600 * 2,
      phi,
      theta: 0.2,
      dark: 0.99,
      diffuse: 0.7,
      mapSamples: 25000,
      mapBrightness: 1,
      baseColor: [0.15, 0.8, 0.4],
      markerColor: [0.1, 0.8, 1],
      glowColor: [0.1, 0.7, 0.3],
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
  }, []);

  return (
    <canvas
      ref={canvasRef}
      style={{ width: 600, height: 600, maxWidth: "100%", aspectRatio: 1 }}
    />
  );
};
