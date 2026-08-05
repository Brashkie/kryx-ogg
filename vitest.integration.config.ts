import { defineConfig } from 'vitest/config'

/**
 * Integration tests: load the REAL native addon (no mock, no setupFiles).
 * Requires `npm run build:native:debug` first.
 */
export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    include: ['__tests__/integration/**/*.test.ts'],
  },
})
