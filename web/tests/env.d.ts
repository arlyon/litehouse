declare namespace Cloudflare {
  interface GlobalProps {
    mainModule: typeof import("./worker");
  }

  interface Env {}
}
