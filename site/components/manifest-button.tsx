import type { SVGProps } from "react";
import { Button, type ButtonProps } from "./ui/button";

export const ManifestButton = (props: ButtonProps & { items?: object[] }) => {
  return (
    <Button className="relative" variant="primary" {...props}>
      <ReceiptTextIcon className="h-5 w-5" />
      {props.items ? (
        <span
          data-count={props.items.length ? "yes" : "no"}
          className="absolute bottom-1 left-2 flex size-4 items-center justify-center rounded-full bg-orange-500 text-xs text-white  data-[count=no]:scale-0 data-[count=no]:rotate-90 scale-100 rotate-0 transition-transform"
        >
          {props.items.length}
        </span>
      ) : null}
    </Button>
  );
};

function ReceiptTextIcon(props: SVGProps<SVGSVGElement>) {
  return (
    <svg
      {...props}
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
      <title>Manifest</title>
      <path d="M4 2v20l2-1 2 1 2-1 2 1 2-1 2 1 2-1 2 1V2l-2 1-2-1-2 1-2-1-2 1-2-1-2 1Z" />
      <path d="M14 8H8" />
      <path d="M16 12H8" />
      <path d="M13 16H8" />
    </svg>
  );
}
