import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    // Unit tests mock the native contract inline (via vi.mock in each file).
    // Integration tests load the real addon and run via a separate config.
    include: ['__tests__/unit/**/*.test.ts'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'lcov', 'html'],
      include: ['src/**/*.ts'],
      // Excluded from UNIT coverage on purpose, covered by the integration
      // suite against the real .node instead:
      //   - native.ts: the addon loader.
      //   - index.ts's coreVersion(): a bridge to the native addon — a mock
      //     would prove only that the wrapper calls the mock, not that the real
      //     addon returns a version. Its value is validated in integration.
      // Unit coverage stays meaningful (SDK logic) rather than inflated.
      exclude: ['src/**/*.d.ts', 'src/native.ts'],
    },
  },
})
