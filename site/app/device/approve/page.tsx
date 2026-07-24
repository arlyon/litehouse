"use client";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { authClient, useSession } from "@/lib/auth-client";
import { useSearchParams, useRouter } from "next/navigation";
import { useState, useEffect } from "react";
import { toast } from "sonner";
import Link from "next/link";

export default function DeviceApprovalPage() {
  const { data: session, isPending } = useSession();
  const searchParams = useSearchParams();
  const router = useRouter();
  const userCode = searchParams.get("user_code");
  const [isProcessing, setIsProcessing] = useState(false);
  const [deviceStatus, setDeviceStatus] = useState<string | null>(null);

  useEffect(() => {
    // Verify the device code is valid
    const verifyCode = async () => {
      if (!userCode) return;

      try {
        const response = await authClient.device({
          query: { user_code: userCode },
        });

        if (response.data) {
          setDeviceStatus(response.data.status);
        }
      } catch (err: any) {
        toast.error("Invalid or expired device code");
        router.push("/device");
      }
    };

    verifyCode();
  }, [userCode, router]);

  const handleApprove = async () => {
    if (!userCode) return;

    setIsProcessing(true);
    try {
      const response = await authClient.device.approve({
        userCode: userCode,
      });

      if (response.data?.success) {
        toast.success("Device approved successfully!");
        router.push("/");
      } else {
        toast.error("Failed to approve device");
      }
    } catch (err: any) {
      toast.error(err?.message || "Failed to approve device");
    } finally {
      setIsProcessing(false);
    }
  };

  const handleDeny = async () => {
    if (!userCode) return;

    setIsProcessing(true);
    try {
      const response = await authClient.device.deny({
        userCode: userCode,
      });

      if (response.data?.success) {
        toast.success("Device denied");
        router.push("/");
      } else {
        toast.error("Failed to deny device");
      }
    } catch (err: any) {
      toast.error(err?.message || "Failed to deny device");
    } finally {
      setIsProcessing(false);
    }
  };

  if (isPending) {
    return (
      <div className="w-full max-w-md space-y-6">
        <Card>
          <CardHeader>
            <CardTitle>Loading...</CardTitle>
          </CardHeader>
        </Card>
      </div>
    );
  }

  if (!session?.user) {
    // Redirect to login if not authenticated
    const redirectUrl = `/device/approve?user_code=${userCode}`;
    return (
      <div className="w-full max-w-md space-y-6">
        <Card>
          <CardHeader>
            <CardTitle>Authentication Required</CardTitle>
            <CardDescription>
              You must be signed in to authorize a device
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Link href={`/sign-in?redirect=${encodeURIComponent(redirectUrl)}`}>
              <Button className="w-full">Sign In</Button>
            </Link>
          </CardContent>
        </Card>
      </div>
    );
  }

  if (!userCode) {
    return (
      <div className="w-full max-w-md space-y-6">
        <Card>
          <CardHeader>
            <CardTitle>Invalid Request</CardTitle>
            <CardDescription>No device code provided</CardDescription>
          </CardHeader>
          <CardContent>
            <Link href="/device">
              <Button className="w-full">Enter Device Code</Button>
            </Link>
          </CardContent>
        </Card>
      </div>
    );
  }

  if (deviceStatus && deviceStatus !== "pending") {
    return (
      <div className="w-full max-w-md space-y-6">
        <Card>
          <CardHeader>
            <CardTitle>Device Already Processed</CardTitle>
            <CardDescription>
              This device has already been {deviceStatus}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Link href="/">
              <Button className="w-full">Go Home</Button>
            </Link>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="w-full max-w-md space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Device Authorization Request</CardTitle>
          <CardDescription>
            A device is requesting access to your account
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="rounded-lg bg-muted p-4 text-center">
            <p className="text-sm text-muted-foreground mb-2">Device Code</p>
            <p className="text-2xl font-mono font-bold tracking-widest">
              {userCode}
            </p>
          </div>

          <div className="rounded-lg border p-4">
            <p className="text-sm text-muted-foreground mb-2">Signed in as</p>
            <p className="font-medium">{session.user.email}</p>
          </div>

          <div className="space-y-2">
            <p className="text-sm text-muted-foreground">
              By approving, this device will have access to your Litehouse account.
            </p>
          </div>

          <div className="flex gap-2">
            <Button
              onClick={handleDeny}
              disabled={isProcessing}
              variant="outline"
              className="flex-1"
            >
              Deny
            </Button>
            <Button
              onClick={handleApprove}
              disabled={isProcessing}
              className="flex-1"
            >
              {isProcessing ? "Processing..." : "Approve"}
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
