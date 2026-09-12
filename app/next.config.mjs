/** @type {import('next').NextConfig} */
const nextConfig = {
  // `@aegis/sdk` (a workspace-local package, `file:../sdk/ts`) ships TypeScript source rather than
  // a pre-built dist -- Next's SWC compiler only transpiles files inside the app itself by
  // default, so it must be told to also transpile this one linked package.
  transpilePackages: ['@aegis/sdk'],
  // `sdk/ts` resolves (via its node_modules symlink) to a real path OUTSIDE `app/`'s own root --
  // Next's webpack config refuses to follow module resolution outside the project directory
  // unless told to. Required for this workspace-local `file:../sdk/ts` dependency to resolve at
  // all (verified empirically: the build fails with "Module not found: Can't resolve '@aegis/sdk'"
  // without this, despite a valid symlink in node_modules).
  experimental: {
    externalDir: true,
  },
  // `@aegis/sdk`'s own relative imports use the `.js`-suffixed NodeNext convention
  // (`./config.js` resolving to the sibling `config.ts`) required for its own standalone
  // typecheck/test toolchain (`tsc --module NodeNext`, `vitest`/`tsx`). Webpack's default
  // resolution does not map that suffix onto a `.ts` source file for an externally-transpiled
  // package, so this alias is required for the build to resolve the SDK's internal imports at all
  // (verified empirically: `--webpack` failed with "Can't resolve './config.js'" etc. without it).
  webpack(config) {
    config.resolve.extensionAlias = {
      ...config.resolve.extensionAlias,
      '.js': ['.ts', '.tsx', '.js'],
    };
    return config;
  },
};

export default nextConfig;
