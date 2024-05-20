import {
  type MouseEventHandler,
  useCallback,
  useEffect,
  useRef,
  useState,
} from "react";

export function useCopyButton(
  onCopy: () => boolean,
): [checked: boolean, error: boolean, onClick: MouseEventHandler] {
  const [success, setSuccess] = useState(false);
  const [error, setError] = useState(false);
  const timeoutRef = useRef<number | null>(null);

  const onClick: MouseEventHandler = useCallback(() => {
    if (timeoutRef.current) window.clearTimeout(timeoutRef.current);
    timeoutRef.current = window.setTimeout(() => {
      setSuccess(false);
      setError(false);
    }, 1500);
    const result = onCopy();
    if (!result) setError(true);
    else setSuccess(true);
  }, [onCopy]);

  // Avoid updates after being unmounted
  useEffect(() => {
    return () => {
      if (timeoutRef.current) window.clearTimeout(timeoutRef.current);
    };
  }, []);

  return [success, error, onClick];
}
