import { CopyBox } from "@/components/copy-box";
import { GithubButton } from "@/components/github-button";
import { RegistryButton } from "@/components/registry-button";
import Link from "next/link";

export default function HomePage() {
  return (
    <main className="flex h-screen flex-col justify-center items-center text-center gap-4">
      <svg
        xmlns="http://www.w3.org/2000/svg"
        className="w-32"
        viewBox="0 0 400 320"
      >
        <path d="M159.75 21.057 120 41.005V71h20v30h-20v40h6c3.3 0 6.025.338 6.056.75.03.412-5.479 31.912-12.244 70l-12.3 69.25H80v39h100v-21.532c0-29.895 4.024-37.424 20-37.424 15.976 0 20 7.529 20 37.424V320h100v-39h-27.512l-12.3-69.25c-6.765-38.088-12.274-69.588-12.244-70 .031-.412 2.756-.75 6.056-.75h6v-40h-20V71h20V41.005l-39.995-20.003c-21.997-11.001-40.11-19.977-40.25-19.948-.14.03-18.142 9.032-40.005 20.003M15.461 15.72c-3.253 7.606-5.351 13.763-4.848 14.229 1.19 1.104 81.073 35.308 82.089 35.148.762-.119 11.038-23.088 11.551-25.82.161-.854-14.311-7.543-41.406-19.138L21.194 2.314 15.461 15.72m321.094 4.71c-21.969 9.48-40.059 17.649-40.2 18.154-.529 1.892 10.362 26.912 11.593 26.634 1.522-.345 82.192-35.024 82.734-35.567 1.034-1.034-11.116-26.645-12.597-26.554-.872.053-19.56 7.853-41.53 17.333M189.75 51.061 180 56.019V101h40V56.018l-9.982-5.009c-5.49-2.755-10.102-4.986-10.25-4.958-.147.028-4.655 2.283-10.018 5.01M51.822 113.864c-22.373 9.7-40.993 17.955-41.377 18.346-.384.39 1.881 6.741 5.034 14.112l5.732 13.403 39.645-16.923c21.804-9.308 40.637-17.327 41.85-17.821 2.408-.98 3.05 1.024-8.233-25.731-.754-1.787-1.506-3.199-1.672-3.136-.166.062-18.606 8.05-40.979 17.75M306.636 97.78C305.713 99.273 296 123.267 296 124.053c0 .436 1.688 1.344 3.75 2.02 2.063.675 20.391 8.36 40.731 17.077 20.34 8.718 37.5 15.85 38.134 15.85.895 0 8.064-15.181 11.947-25.301.49-1.276-55.577-26.299-77.562-34.617-6.387-2.417-5.755-2.287-6.364-1.302" />
      </svg>
      <h1 className="text-2xl font-bold">Litehouse</h1>
      <p className="text-muted-foreground">
        View the{" "}
        <Link href="/docs" className="text-foreground font-semibold underline">
          /docs
        </Link>{" "}
        or see the source code and package registry.
      </p>
      <CopyBox className="my-4" command="cargo install litehouse" />
      <div className="flex gap-4">
        <Link href="https://github.com/arlyon/litehouse">
          <GithubButton />
        </Link>
        <Link href="/registry">
          <RegistryButton />
        </Link>
      </div>
    </main>
  );
}
