//@ts-check
const { resolve } = require('node:path');
const { build } = require('esbuild');

/**
 * - ref: https://github.com/evanw/esbuild/issues/1051#issuecomment-806325487
 * @satisfies {import('esbuild').Plugin }
 */
const nativeNodeModulesPlugin = /** @type {const} */ ({
  name: 'native-node-modules',
  setup(build) {
    // If a ".node" file is imported within a module in the "file" namespace, resolve
    // it to an absolute path and put it into the "node-file" virtual namespace.
    build.onResolve({ filter: /\.node$/, namespace: 'file' }, (args) => {
      try {
        return {
          path: require.resolve(args.path, { paths: [args.resolveDir] }),
          namespace: 'node-file',
        };
      } catch {
        // Skip missing native module at build time
        return { path: args.path, external: true };
      }
    });

    // Files in the "node-file" virtual namespace call "require()" on the
    // path from esbuild of the ".node" file in the output directory.
    build.onLoad({ filter: /.*/, namespace: 'node-file' }, (args) => ({
      contents: `
        import path from ${JSON.stringify(args.path)}
        try { module.exports = require(path) }
        catch {}
      `,
    }));

    // If a ".node" file is imported within a module in the "node-file" namespace, put
    // it in the "file" namespace where esbuild's default loading behavior will handle
    // it. It is already an absolute path since we resolved it to one above.
    build.onResolve({ filter: /\.node$/, namespace: 'node-file' }, (args) => ({
      path: args.path,
      namespace: 'file',
    }));

    // Tell esbuild's default loading behavior to use the "file" loader for
    // these ".node" files.
    let opts = build.initialOptions;
    opts.loader = opts.loader || {};
    opts.loader['.node'] = 'file';
  },
});

/**
 * Shared options
 *
 * @satisfies {import('esbuild').BuildOptions }
 */
const sharedOptions = /** @type {const} */ ({
  plugins: [nativeNodeModulesPlugin],
  bundle: true,
  platform: 'node', // Electron main/preload are Node
  target: 'node18', // Adjust to Electron version
  minify: process.env.NODE_ENV === 'release',
  external: [
    'electron',
    'node:*', // Node built-ins
    // 'd_merge_node', // native module
  ],
});

// Build main process
build({
  ...sharedOptions,
  entryPoints: [resolve(__dirname, 'src/main.ts')],
  outfile: resolve(__dirname, 'out/main.js'),
  format: 'cjs',
}).catch(() => process.exit(1));

// Build preload script
build({
  ...sharedOptions,
  entryPoints: [resolve(__dirname, 'src/preload.ts')],
  outfile: resolve(__dirname, 'out/preload.js'),
  format: 'cjs',
}).catch(() => process.exit(1));
