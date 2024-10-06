import { ogFetch } from "@/lib/opengraph-fetch";
import Image from "next/image";
import Link from "next/link";

export async function OpenGraph({ href }: { href: string }) {
  const data = await ogFetch(href);
  if ("error" in data) {
    throw new Error(
      `failed to get opengraph for ${href}: ${JSON.stringify(data.error)}`,
    );
  }

  const url = data.url.replace("https://", "").split("/")[0];

  return (
    <Link href={href} className="not-prose" target="_blank">
      <div className="shadow-lg flex flex-col lg:h-[200px] lg:flex-row bg-primary-foreground outline outline-1 outline-fd-border">
        <div className="relative w-full lg:h-full aspect-[1.91/1]">
          <Image src={data.image} fill={true} className="relative my-0" />
        </div>
        <div className="p-4 lg:border-l border-t lg:border-t-0 overflow-y-scroll">
          <div className="text-muted-foreground text-xs uppercase">{url}</div>
          <div className="font-bold text-lg mb-2 truncate">{data.title}</div>
          <p className="text-muted-foreground text-xs">{data.description}</p>
        </div>
      </div>
    </Link>
  );
}
