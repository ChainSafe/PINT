/**
 * E2E tests for PINT
 */

/**
 * Testcase
 */
interface Tests {
    [fn: string]: (...args: any[]) => Promise<void>;
}

// Run tests
const run = async (tests: Tests): Promise<void> => {};
