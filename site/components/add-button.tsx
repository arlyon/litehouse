"use client";

import { useManifestStore } from "@/hooks/use-indexed-db";
import type { SVGProps } from "react";
import { Button } from "./ui/button";

export const AddButton = ({
  name,
  version,
}: { name: string; version: string }) => {
  const id = `${name}@${version}`;
  const { add, items, remove } = useManifestStore<{ id: string }>();

  const exists = items?.some((item) => item.id === id);

  return exists ? (
    <Button
      data-selected="true"
      size="sm"
      variant="primary"
      onClick={() => remove(id)}
    >
      <MinusIcon className="mr-2 h-4 w-4" />
      Del
    </Button>
  ) : (
    <Button
      data-selected="false"
      size="sm"
      variant="primary"
      onClick={() => add({ id })}
    >
      <PlusIcon className="mr-2 h-4 w-4" />
      Add
    </Button>
  );
};

function PlusIcon(props: SVGProps<SVGSVGElement>) {
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
      <title>Plus</title>
      <path d="M5 12h14" />
      <path d="M12 5v14" />
    </svg>
  );
}

function MinusIcon(props: SVGProps<SVGSVGElement>) {
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
      <title>Minus</title>
      <path d="M5 12h14" />
    </svg>
  );
}
