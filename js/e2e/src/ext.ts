/**
 * Runner extensions
 */
import { ISubmittableResult } from "@polkadot/types/types";

/**
 * Wait for n blocks
 */
export async function waitBlock(block: number) {}

/**
 * Timeout for promise
 */
export async function timeout<T>(fn: Promise<T>, ms?: number): Promise<T> {
    return fn;
}

/**
 * Exit process if failed
 */
export async function checkError(sr: ISubmittableResult) {
    process.exit(1);
}
