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
