import Link from "next/link";
import "./page.css";
import {
  BoxIcon,
  BrainIcon,
  CloudLightning,
  GithubIcon,
  LockIcon,
  PartyPopper,
  Plug,
  Text,
} from "lucide-react";
import { Button } from "@/components/ui/button";

export const Logo = () => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    className="w-10"
    viewBox="0 0 400 320"
  >
    <title>Logo</title>
    <path
      fill="currentcolor"
      d="M159.75 21.057 120 41.005V71h20v30h-20v40h6c3.3 0 6.025.338 6.056.75.03.412-5.479 31.912-12.244 70l-12.3 69.25H80v39h100v-21.532c0-29.895 4.024-37.424 20-37.424 15.976 0 20 7.529 20 37.424V320h100v-39h-27.512l-12.3-69.25c-6.765-38.088-12.274-69.588-12.244-70 .031-.412 2.756-.75 6.056-.75h6v-40h-20V71h20V41.005l-39.995-20.003c-21.997-11.001-40.11-19.977-40.25-19.948-.14.03-18.142 9.032-40.005 20.003M15.461 15.72c-3.253 7.606-5.351 13.763-4.848 14.229 1.19 1.104 81.073 35.308 82.089 35.148.762-.119 11.038-23.088 11.551-25.82.161-.854-14.311-7.543-41.406-19.138L21.194 2.314 15.461 15.72m321.094 4.71c-21.969 9.48-40.059 17.649-40.2 18.154-.529 1.892 10.362 26.912 11.593 26.634 1.522-.345 82.192-35.024 82.734-35.567 1.034-1.034-11.116-26.645-12.597-26.554-.872.053-19.56 7.853-41.53 17.333M189.75 51.061 180 56.019V101h40V56.018l-9.982-5.009c-5.49-2.755-10.102-4.986-10.25-4.958-.147.028-4.655 2.283-10.018 5.01M51.822 113.864c-22.373 9.7-40.993 17.955-41.377 18.346-.384.39 1.881 6.741 5.034 14.112l5.732 13.403 39.645-16.923c21.804-9.308 40.637-17.327 41.85-17.821 2.408-.98 3.05 1.024-8.233-25.731-.754-1.787-1.506-3.199-1.672-3.136-.166.062-18.606 8.05-40.979 17.75M306.636 97.78C305.713 99.273 296 123.267 296 124.053c0 .436 1.688 1.344 3.75 2.02 2.063.675 20.391 8.36 40.731 17.077 20.34 8.718 37.5 15.85 38.134 15.85.895 0 8.064-15.181 11.947-25.301.49-1.276-55.577-26.299-77.562-34.617-6.387-2.417-5.755-2.287-6.364-1.302"
    />
  </svg>
);

const LogoIcon = () => (
  <a
    href="/"
    aria-current="page"
    className="rounded-2xl justify-center items-center p-[1px] flex relative overflow-hidden hover:scale-105 transition-transform"
  >
    <div className="animate-[spin_5s_linear_infinite] duration-500 absolute bg-gradient-to-br from-green-600 to-green-400 dark:from-green-950 dark:to-green-500 size-20"></div>
    <div className="relative text-white bg-gradient-to-br from-green-600 to-green-400 dark:from-black dark:to-green-800 p-3 aspect-square rounded-2xl flex items-center justify-center">
      <Logo />
    </div>
  </a>
);

export default function HomePage() {
  return (
    <>
      <div className="main-grid grid w-screen grid-cols-[auto_10px_minmax(min-content,_max-content)_10px_auto] md:grid-cols-[auto_50px_minmax(min-content,_max-content)_50px_auto] gap-[1px] bg-border dark:bg-muted">
        <div />
        <div />
        <div className="flex items-center p-4 lg:p-8 justify-center">
          <LogoIcon />
        </div>
        <div />
        <div />
        <div />
        <div />
        <div className="h-[50px]"></div>
        <div />
        <div />
        <div />
        <div />
        <div className="bg-gradient-to-tl dark:from-black dark:to-green-950 flex relative flex-col gap-6 items-center justify-center bg-background p-4 lg:p-16 outline outline-1 outline-green-500 dark:outline-green-600">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="currentColor"
            stroke="none"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="absolute -left-2 -top-2 z-10 size-4 xl:scale-[200%]"
          >
            <path d="M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z"></path>
            <path d="M20 3v4"></path>
            <path d="M22 5h-4"></path>
            <path d="M4 17v2"></path>
            <path d="M5 18H3"></path>
          </svg>
          <Link
            href="/registry"
            className="border rounded-full px-4 py-2 text-xs dark:bg-green-950 dark:border-green-800 dark:text-green-500 flex flex-row gap-2 items-center"
          >
            <PartyPopper className="size-4" /> The Registry is now live. Check
            it out!
          </Link>
          <h1 className="text-4xl sm:text-6xl md:text-8xl font-bold text-center">
            <span className="inline-block">Home Automation</span>{" "}
            <span className="inline-block">
              That{" "}
              <span className="bg-gradient-to-b from-green-950 to-green-500 inline-block text-transparent bg-clip-text">
                Scales
              </span>
            </span>
          </h1>
          <div className="flex flex-row flex-wrap gap-4 items-center justify-center mt-4">
            <Link href="/registry">
              <Button
                className="inline-flex items-center gap-2"
                variant="primary"
              >
                <BoxIcon className="h-5 w-5" />
                Package Registry
              </Button>
            </Link>
            <Link href="/docs">
              <Button
                className="inline-flex items-center gap-2"
                variant="secondary"
              >
                <Text className="h-5 w-5" />
                Documentation
              </Button>
            </Link>
            <Link
              href="https://github.com/arlyon/litehouse"
              className="hidden lg:block"
            >
              <Button
                className="inline-flex items-center gap-2"
                variant="outline"
              >
                <GithubIcon className="h-5 w-5" />
                GitHub
              </Button>
            </Link>
          </div>
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="currentColor"
            stroke="none"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            className="absolute -right-2 -bottom-2 z-10 size-4 xl:scale-[200%]"
          >
            <path d="M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z"></path>
            <path d="M20 3v4"></path>
            <path d="M22 5h-4"></path>
            <path d="M4 17v2"></path>
            <path d="M5 18H3"></path>
          </svg>
        </div>
        <div />
        <div />
        <div />
        <div />
        <div className="p-16 gap-16 flex items-center justify-center">
          <p className="text-lg max-w-[750px] font-mono bg-green-500 inline-block text-transparent bg-clip-text">
            Litehouse is a lightweight OS for your home, designed from the
            ground up to be easy to use, resource efficient, open, and secure.
          </p>
        </div>
        <div />
        <div />

        <div />
        <div />
        <div className="flex items-center justify-center pattern-diagonal-lines pattern-green-300 pattern-bg-green-50 dark:pattern-bg-black pattern-size-2 pattern-opacity-100">
          <div className="main-grid grid gap-[1px] sm:grid-cols-2 xl:grid-cols-4 bg-border outline outline-1 outline-border dark:outline-muted dark:bg-muted">
            {[
              {
                icon: Plug,
                title: "Open Plugins",
                desc: "An open registry means anyone can create and publish plugins.",
              },
              {
                icon: CloudLightning,
                title: "Resource Efficient",
                desc: "Plugins are extremely lightweight and can run on even the smallest devices.",
              },
              {
                icon: LockIcon,
                title: "Secure",
                desc: "Plugins are fully sandboxed from your system, and the network, unless allowed.",
              },
              {
                icon: BrainIcon,
                title: "Ergonomic",
                desc: "We built the system from the ground up to be easy to configure. No more yaml.",
              },
            ].map((data, idx) => (
              <HomeTile key={idx} {...data} />
            ))}
          </div>
        </div>
        <div />
        <div />
        <div />
        <div />
        <div className="h-[50px]" />
        <div />
        <div />
        <div />
        <div />
        <div className="flex flex-col items-center p-16">
          <h2 className="text-4xl text-center">
            Integrate <u>Everything</u>
          </h2>
        </div>
        <div />
        <div />
      </div>
    </>
  );
}

const HomeTile = ({ title, desc, icon: Icon }) => (
  <div className="flex flex-col items-start justify-center gap-4 p-6 max-w-80">
    <div className="bg-neutral-100 p-3 rounded-md dark:bg-neutral-800">
      <Icon className="w-6 h-6" />
    </div>
    <h3 className="text-xl font-bold">{title}</h3>
    <p className="text-neutral-500 dark:text-neutral-400">{desc}</p>
  </div>
);
