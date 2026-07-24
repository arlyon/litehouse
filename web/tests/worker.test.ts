import { exports } from "cloudflare:workers";
import { describe, expect, it } from "vitest";

describe("Cloudflare runtime gate", () => {
  it("serves the built TanStack application inside workerd", async () => {
    const response = await exports.default.fetch(
      new Request("https://litehouse.test/"),
    );
    const html = await response.text();

    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toContain("text/html");
    expect(html).toContain("Nine typed capability facets");
    expect(html).toContain("Greenfield Worker gate");
    expect(html).toContain('href="#main-content"');
  });
});
