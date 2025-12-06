"use client";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { authClient } from "@/lib/auth-client";
import { useState, useEffect } from "react";
import { toast } from "sonner";
import { useRouter, useSearchParams } from "next/navigation";

export default function DeviceAuthorizationPage() {
  const searchParams = useSearchParams();
  const urlUserCode = searchParams.get("user_code");
  const [userCode, setUserCode] = useState("");
  const [loading, setLoading] = useState(false);
  const router = useRouter();

  // If user_code is in URL, auto-verify and redirect
  useEffect(() => {
    const autoVerify = async () => {
      if (!urlUserCode) return;

      const formattedCode = urlUserCode.trim().replace(/-/g, "").toUpperCase();
      setUserCode(formatUserCodeInput(formattedCode));
      setLoading(true);

      try {
        const response = await authClient.device({
          query: { user_code: formattedCode },
        });

        if (response.data) {
          // Redirect to approval page
          router.push(`/device/approve?user_code=${formattedCode}`);
        }
      } catch (err: any) {
        toast.error(err?.message || "Invalid or expired code");
      } finally {
        setLoading(false);
      }
    };

    autoVerify();
  }, [urlUserCode, router]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);

    try {
      // Format the code: remove dashes and convert to uppercase
      const formattedCode = userCode.trim().replace(/-/g, "").toUpperCase();

      // Check if the code is valid using GET /device endpoint
      const response = await authClient.device({
        query: { user_code: formattedCode },
      });

      if (response.data) {
        // Redirect to approval page
        router.push(`/device/approve?user_code=${formattedCode}`);
      }
    } catch (err: any) {
      toast.error(err?.message || "Invalid or expired code");
    } finally {
      setLoading(false);
    }
  };

  const formatUserCodeInput = (value: string) => {
    // Remove all non-alphanumeric characters
    const cleaned = value.replace(/[^A-Za-z0-9]/g, "").toUpperCase();

    // Add hyphen after 4 characters
    if (cleaned.length <= 4) {
      return cleaned;
    }
    return `${cleaned.slice(0, 4)}-${cleaned.slice(4, 8)}`;
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const formatted = formatUserCodeInput(e.target.value);
    setUserCode(formatted);
  };

  return (
    <div className="w-full max-w-md space-y-6">
      <div className="space-y-2 text-center">
        <h1 className="text-3xl font-bold">Device Authorization</h1>
        <p className="text-muted-foreground">
          Enter the code displayed on your device
        </p>
      </div>

      <form onSubmit={handleSubmit} className="space-y-4">
        <div className="space-y-2">
          <Label htmlFor="userCode">Device Code</Label>
          <Input
            id="userCode"
            type="text"
            placeholder="ABCD-1234"
            value={userCode}
            onChange={handleInputChange}
            maxLength={9}
            className="text-center text-2xl font-mono tracking-widest"
            required
          />
          <p className="text-xs text-muted-foreground">
            Enter the 8-character code shown on your device
          </p>
        </div>

        <Button type="submit" className="w-full" disabled={loading || userCode.length < 9}>
          {loading ? "Verifying..." : "Continue"}
        </Button>
      </form>
    </div>
  );
}
