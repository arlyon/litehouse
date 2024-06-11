import { ImageResponse } from "next/og";
import { Logo, Star } from "../../page";
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
export default async function Image({ params }: { title: string }) {
  // Font
  const interBold = fetch(
    new URL("../../../public/fonts/Inter-Bold.ttf", import.meta.url),
  ).then((res) => res.arrayBuffer());
  const interRegular = fetch(
    new URL("../../../public/fonts/Inter-Regular.ttf", import.meta.url),
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
          <span>Litehouse</span>
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
          {params.title}
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
