"use client";

import { useEffect, useMemo, useState } from "react";
import { useParams, useRouter } from "next/navigation";
import queryString from "query-string";

// How do I get the pathname with hash.
// source: https://github.com/vercel/next.js/discussions/49465
export const useHashState = ({ scroll }: { scroll?: boolean }) => {
  const getCurrentHash = useMemo(
    () => () =>
      typeof window !== "undefined"
        ? window.location.hash.replace(/^#!?/, "")
        : "",
    [],
  );
  const router = useRouter();
  const params = useParams();
  const [hash, _setHash] = useState<string | null>(getCurrentHash());

  const setHash = (newHash: string | null) => {
    let updatedUrl = window.location.href;
    updatedUrl = queryString.stringifyUrl({
      url: updatedUrl.split("#")[0],
      fragmentIdentifier: newHash,
    });

    _setHash(newHash);
    router.replace(updatedUrl, { scroll });
  };
  useEffect(() => {
    const currentHash = getCurrentHash();
    _setHash(currentHash);
  }, [params]);

  const handleHashChange = () => {
    const currentHash = getCurrentHash();
    _setHash(currentHash);
  };

  useEffect(() => {
    window.addEventListener("hashchange", handleHashChange);

    return () => {
      window.removeEventListener("hashchange", handleHashChange);
    };
  }, []);

  return [hash, setHash] as const;
};
