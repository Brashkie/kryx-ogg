import { defineConfig } from 'tsup'

/**
 * Pattern proven by @kryxjs/core and @kryxjs/codecs-opus:
 *   - No `shims: true` (it generates a broken __require helper).
 *   - The native addon is loaded via `import * as addon from '../index.js'`,
 *     which becomes a literal `require('../index.js')` in CJS output and a
 *     literal `import` in ESM — both supported natively by Node.
 *   - `external` keeps tsup from bundling the native loader or the binary.
 */
export default defineConfig({
  entry: ['src/index.ts'],
  format: ['cjs', 'esm'],
  dts: true,
  splitting: false,
  sourcemap: true,
  clean: true,
  minify: false,
  shims: false,
  target: 'node18',
  outDir: 'dist',
  // The native addon loader and binary stay outside the bundle.
  external: ['../index.js', '../index.cjs', /\.node$/, /^@kryxjs\/ogg-/],
  outExtension({ format }) {
    return { js: format === 'cjs' ? '.cjs' : '.mjs' }
  },
})
