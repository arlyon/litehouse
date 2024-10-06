import { getPage, getPages } from "@/app/source";
import { DocsBody, DocsPage } from "fumadocs-ui/page";
import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { Edit } from "lucide-react";
import defaultMdxComponents from "fumadocs-ui/mdx";
import { CodeBlock, Pre } from "@/components/code-block";

export default async function Page({
  params,
}: {
  params: { slug?: string[] };
}) {
  const page = getPage(params.slug);

  if (page == null) {
    notFound();
  }

  const path = `site/content/docs/${page.file.path}`;

  const footer = (
    <a
      href={`https://github.com/arlyon/litehouse/blob/main/${path}`}
      target="_blank"
      rel="noreferrer noopener"
      className="inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
    >
      <Edit className="me-2 size-4" />
      Edit on Github
    </a>
  );

  const MDX = page.data.body;

  console.log(page);

  return (
    <DocsPage
      toc={page.data.toc}
      tableOfContent={{
        enabled: !!page.data.toc,
        style: "clerk",
        footer,
      }}
      tableOfContentPopover={{ footer }}
    >
      <DocsBody>
        <h1 className="mb-0">{page.data.title}</h1>
        <p className="mt-0 text-purple-600">{page.data.description}</p>
        <MDX
          components={{
            ...defaultMdxComponents,
            pre: ({ ref: _ref, icon, title, ...props }: any) => (
              <CodeBlock
                icon={icon}
                title={title}
                allowDL={title?.includes(".")}
              >
                <Pre {...props} />
              </CodeBlock>
            ),
          }}
        />
      </DocsBody>
    </DocsPage>
  );
}

export async function generateStaticParams() {
  return getPages().map((page) => ({
    slug: page.slugs,
  }));
}

export function generateMetadata({ params }: { params: { slug?: string[] } }) {
  const page = getPage(params.slug);

  if (page == null) notFound();

  console.log(page);

  return {
    title: page.data.title,
    description: page.data.description,
  } satisfies Metadata;
}
