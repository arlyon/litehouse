import { useEffect, useState } from "react";

export const useNotNull = <T>(value: T | null) => {
  const [notNull, setNotNull] = useState<T | null>(value);

  useEffect(() => {
    if (value !== null) {
      setNotNull(value);
    }
  }, [value]);

  return notNull;
};
