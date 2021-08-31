/**
 * Utils
 */
import { IExtrinsic } from "./config";

export function assert(r: boolean, msg: string): string | void {
    if (!r) {
        return msg;
    }
}

/**
 * Expand Id of extrinsic
 */
export function expandId(e: IExtrinsic): IExtrinsic {
    if (!e.id) e.id = `${e.pallet}.${e.call}`;

    if (e.with) {
        for (const r of e.with) {
            if (typeof r !== "function") {
                expandId(r);
            }
        }
    }

    return e;
}

/**
 * Wait for n blocks
 *
 * The current gap of producing a block is 4s,
 * we use 5s here.
 *
 * @param {number} block
 * @returns {Promise<void>}
 */
export async function waitBlock(block: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, block * 12000));
}
