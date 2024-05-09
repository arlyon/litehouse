import type { MDXComponents } from "mdx/types";
import defaultComponents from "fumadocs-ui/mdx";
import { Pre, CodeBlock } from "@/components/code-block";

export function useMDXComponents(components: MDXComponents): MDXComponents {
  return {
    ...defaultComponents,
    ...components,
    pre: ({ ref: _ref, icon, title, ...props }: any) =>
       (
         <CodeBlock icon={icon} title={title} allowDL={title?.includes(".")}>
          <Pre {...props} />
        </CodeBlock>
      )

  };
}
