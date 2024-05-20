"use client";
import {
  ScrollArea,
  ScrollBar,
  ScrollViewport,
} from "@/components/scroll-area";
import { useCopyButton } from "@/hooks/use-copy-button";
import {
  CheckIcon,
  CopyIcon,
  CrossIcon,
  DownloadIcon,
  XIcon,
} from "lucide-react";
import type { ButtonHTMLAttributes, HTMLAttributes, ReactNode } from "react";
import { forwardRef, useCallback, useRef } from "react";
import { twMerge as cn } from "tailwind-merge";
import { buttonVariants } from "./ui/button";

export type CodeBlockProps = HTMLAttributes<HTMLElement> & {
  /**
   * Icon of code block
   */
  icon?: ReactNode;

  allowCopy?: boolean;
  allowDL?: boolean;
};

export const Pre = forwardRef<HTMLPreElement, HTMLAttributes<HTMLPreElement>>(
  ({ className, ...props }, ref) => {
    return (
      <pre ref={ref} className={cn("nd-codeblock py-4", className)} {...props}>
        {props.children}
      </pre>
    );
  },
);

Pre.displayName = "Pre";

/**
 * Create a dummy `a` tag with a data URI matching the given text, and click it.
 */
const download = (text: string, filename?: string) => {
  const element = document.createElement("a");
  element.setAttribute(
    "href",
    `data:text/plain;charset=utf-8,${encodeURIComponent(text)}`,
  );
  if (filename) {
    element.setAttribute("download", filename);
  }

  element.style.display = "none";
  document.body.appendChild(element);

  element.click();

  document.body.removeChild(element);
};

export const CodeBlock = forwardRef<HTMLElement, CodeBlockProps>(
  (
    {
      title,
      allowCopy = true,
      allowDL = false,
      icon,
      className,
      children,
      ...props
    },
    ref,
  ) => {
    const areaRef = useRef<HTMLDivElement>(null);
    const onCopy = useCallback(() => {
      const pre = areaRef.current?.getElementsByTagName("pre").item(0);

      if (!pre) return;

      const clone = pre.cloneNode(true) as HTMLElement;
      for (const node of clone.querySelectorAll(".nd-copy-ignore")) {
        node.remove();
      }

      void navigator.clipboard.writeText(clone.textContent ?? "");
    }, []);

    const onSave = useCallback(() => {
      // get text from `areaRef`
      if (areaRef.current) {
        download(areaRef.current.innerText, title);
      }
    }, [title]);

    return (
      <figure
        ref={ref}
        className={cn(
          "not-prose group relative my-6 overflow-hidden rounded-lg border bg-secondary/50 text-sm",
          className,
        )}
        {...props}
      >
        {title ? (
          <div className="flex flex-row items-center gap-2 border-b bg-muted px-4 py-1.5">
            <div className="text-muted-foreground [&_svg]:size-3.5">{icon}</div>
            <figcaption className="flex-1 truncate text-muted-foreground">
              {title}
            </figcaption>
            {allowCopy ? (
              <CopyButton className="-mr-2" onCopy={onCopy} />
            ) : null}
            {allowDL ? <SaveButton className="-mr-2" onSave={onSave} /> : null}
          </div>
        ) : (
          <>
            {allowCopy && (
              <CopyButton
                className="absolute right-2 top-2 z-[2] backdrop-blur-sm"
                onCopy={onCopy}
              />
            )}
            {allowDL && (
              <SaveButton
                className="absolute right-2 top-2 z-[2] backdrop-blur-sm"
                onSave={onSave}
              />
            )}
          </>
        )}
        <ScrollArea ref={areaRef} dir="ltr">
          <ScrollViewport>{children}</ScrollViewport>
          <ScrollBar orientation="horizontal" />
        </ScrollArea>
      </figure>
    );
  },
);

CodeBlock.displayName = "CodeBlock";

export function SaveButton({
  className,
  onSave,
  ...props
}: {
  className?: string;
  onSave: () => void;
}) {
  const [checked, onClick] = useCopyButton(onSave);

  return (
    <button
      type="button"
      className={cn(
        buttonVariants({
          variant: "ghost",
          className: "transition-all group-hover:opacity-100",
        }),
        !checked && "opacity-0",
        className,
      )}
      aria-label="Copy Text"
      onClick={onClick}
      {...props}
    >
      <CheckIcon
        className={cn("size-3.5 transition-transform", !checked && "scale-0")}
      />
      <DownloadIcon
        className={cn(
          "absolute size-3.5 transition-transform",
          checked && "scale-0",
        )}
      />
    </button>
  );
}

export function CopyButton({
  className,
  onCopy,
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & {
  onCopy: () => boolean;
}): React.ReactElement {
  const [success, error, onClick] = useCopyButton(onCopy);

  return (
    <button
      type="button"
      className={cn(
        buttonVariants({
          variant: "ghost",
          className: "transition-all group-hover:opacity-100",
        }),
        !success && "opacity-0",
        className,
      )}
      aria-label="Copy Text"
      onClick={onClick}
      {...props}
    >
      <CheckIcon
        className={cn("size-3.5 transition-transform", !success && "scale-0")}
      />
      <XIcon
        className={cn(
          "absolute size-3.5 transition-transform text-red-500",
          !error && "scale-0",
        )}
      />
      <CopyIcon
        className={cn(
          "absolute size-3.5 transition-transform",
          success && "scale-0",
          error && "scale-0",
        )}
      />
    </button>
  );
}
