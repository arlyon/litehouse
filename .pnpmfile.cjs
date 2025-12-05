function readPackage(pkg) {
  // this package is needed but not included in the deps
  if (pkg.name === "better-auth") {
    pkg.dependencies = {
      ...pkg.dependencies,
      "drizzle-orm": "^0.45.0",
    };
  }

  if (pkg.name === "drizzle-kit") {
    pkg.dependencies = {
      ...pkg.dependencies,
      "drizzle-orm": "^0.45.0",
      "@libsql/client": "^0.15.15",
    };
  }

  return pkg;
}

module.exports = {
  hooks: {
    readPackage,
  },
};
