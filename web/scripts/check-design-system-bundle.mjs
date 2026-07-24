import { createHash } from "node:crypto";
import { lstat, readFile, readdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const webDirectory = path.resolve(scriptDirectory, "..");
const bundleDirectory = path.join(
  webDirectory,
  "vendor",
  "open-design",
  "litehouse",
);
const lockPath = path.join(
  webDirectory,
  "vendor",
  "open-design",
  "litehouse.lock.json",
);
const sourcePackageSha256 =
  "a89667c2c1ed7ebfb7f23ac1acb03310706581810a1a0b14fa7c663827c63105";

async function collectFiles(directory, relativeDirectory = "") {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];

  for (const entry of entries.sort((left, right) =>
    left.name < right.name ? -1 : left.name > right.name ? 1 : 0,
  )) {
    const relativePath = path.posix.join(relativeDirectory, entry.name);
    const absolutePath = path.join(directory, entry.name);
    const stats = await lstat(absolutePath);

    if (stats.isSymbolicLink()) {
      throw new Error(`Vendored design-system bundle contains a symlink: ${relativePath}`);
    }

    if (stats.isDirectory()) {
      files.push(...(await collectFiles(absolutePath, relativePath)));
      continue;
    }

    if (!stats.isFile()) {
      throw new Error(`Unsupported vendored bundle entry: ${relativePath}`);
    }

    const bytes = await readFile(absolutePath);
    files.push({
      path: relativePath,
      bytes: bytes.byteLength,
      sha256: createHash("sha256").update(bytes).digest("hex"),
    });
  }

  return files;
}

function createLock(files) {
  const canonicalFiles = files
    .map((file) => `${file.path}\0${file.bytes}\0${file.sha256}\n`)
    .join("");

  return {
    schemaVersion: 2,
    generatedBy: "check-design-system-bundle.mjs@1",
    source: {
      project: "open-design",
      designSystem: "litehouse",
      provenance: "closed-reviewed-export",
      packageSha256: sourcePackageSha256,
    },
    bundleSha256: createHash("sha256").update(canonicalFiles).digest("hex"),
    files,
  };
}

const expected = createLock(await collectFiles(bundleDirectory));
const serialized = `${JSON.stringify(expected, null, 2)}\n`;

if (process.argv.includes("--write")) {
  await writeFile(lockPath, serialized, "utf8");
  process.stdout.write(
    `Wrote ${path.relative(webDirectory, lockPath)} (${expected.bundleSha256}).\n`,
  );
} else {
  const actual = await readFile(lockPath, "utf8");
  if (actual !== serialized) {
    throw new Error(
      "Vendored Litehouse Open Design bundle differs from litehouse.lock.json. Review the source snapshot, then run pnpm bundle:write.",
    );
  }
  process.stdout.write(
    `Verified Litehouse Open Design bundle ${expected.bundleSha256}.\n`,
  );
}
