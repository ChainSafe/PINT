/**
 * E2E tests for PINT
 */

/**
 * Extrinsic definition
 */
export interface Extrinsic {
    pallet: string;
    call: string;
    args: any[];
    timeout?: undefined | number;
}

/**
 * Traverse all extrinsics
 */
export async function run(exs: Extrinsic[]) {}
