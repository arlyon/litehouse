import { ImageResponse } from "next/og";
import { Logo, Star } from "./page";
import { PartyPopper } from "lucide-react";

// Route segment config
export const runtime = "edge";

// Image metadata
export const alt = "About Acme";
export const size = {
  width: 1200,
  height: 630,
};

export const contentType = "image/png";

// Image generation
export default async function Image() {
  // Font
  const interBold = fetch(
    new URL("../public/fonts/Inter-Bold.ttf", import.meta.url),
  ).then((res) => res.arrayBuffer());
  const interRegular = fetch(
    new URL("../public/fonts/Inter-Regular.ttf", import.meta.url),
  ).then((res) => res.arrayBuffer());

  return new ImageResponse(
    (
      <div
        style={{
          background: "black",
          position: "relative",
          color: "white",
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          paddingTop: 20,
          gap: 20,
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        <Star
          style={{
            height: 60,
            width: 60,
            top: 50,
            left: 50,
            text: "white",
            position: "absolute",
          }}
        />
        <Logo style={{ width: 200, height: 160 }} />
        <div
          style={{
            display: "flex",
            fontWeight: 700,
            fontSize: 80,
            flexDirection: "column",
            justifyContent: "center",
            alignItems: "center",
          }}
        >
          <span>Home Automation</span>
          <span>
            That<span style={{ color: "#22c55e", marginLeft: 14 }}>Scales</span>
          </span>
        </div>
        <div
          style={{
            display: "flex",
            flexDirection: "row",
            justifyContent: "center",
            gap: 10,
            marginTop: 20,
            alignItems: "center",
            fontSize: 26,
            border: "1px solid #22c55e",
            color: "#22c55e",
            borderRadius: 9999,
            paddingTop: 14,
            paddingBottom: 14,
            paddingLeft: 24,
            paddingRight: 24,
          }}
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M5.8 11.3 2 22l10.7-3.79" />
            <path d="M4 3h.01" />
            <path d="M22 8h.01" />
            <path d="M15 2h.01" />
            <path d="M22 20h.01" />
            <path d="m22 2-2.24.75a2.9 2.9 0 0 0-1.96 3.12c.1.86-.57 1.63-1.45 1.63h-.38c-.86 0-1.6.6-1.76 1.44L14 10" />
            <path d="m22 13-.82-.33c-.86-.34-1.82.2-1.98 1.11c-.11.7-.72 1.22-1.43 1.22H17" />
            <path d="m11 2 .33.82c.34.86-.2 1.82-1.11 1.98C9.52 4.9 9 5.52 9 6.23V7" />
            <path d="M11 13c1.93 1.93 2.83 4.17 2 5-.83.83-3.07-.07-5-2-1.93-1.93-2.83-4.17-2-5 .83-.83 3.07.07 5 2Z" />
          </svg>
          The Registry is now live. Check it out!
        </div>
        <Star
          style={{
            height: 60,
            width: 60,
            bottom: 50,
            right: 50,
            text: "white",
            position: "absolute",
          }}
        />
      </div>
    ),
    // ImageResponse options
    {
      // For convenience, we can re-use the exported opengraph-image
      // size config to also set the ImageResponse's width and height.
      ...size,
      fonts: [
        {
          name: "Inter",
          data: await interBold,
          style: "normal",
          weight: 700,
        },
        {
          name: "Inter",
          data: await interRegular,
          style: "normal",
          weight: 400,
        },
      ],
    },
  );
}
