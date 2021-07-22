/**
 * Utils
 */
import { Extrinsic } from "./config";

export function assert(r: boolean, msg: string): string | void {
    if (!r) {
        return msg;
    }
}

/**
 * Expand Id of extrinsic
 */
export function expandId(e: Extrinsic): Extrinsic {
    if (!e.id) e.id = `${e.pallet}.${e.call}`;

    for (const r in e.required) {
        if (typeof r !== "string" && typeof r !== "function") {
            expandId(r);
        }
    }

    for (const r in e.with) {
        if (typeof r !== "string" && typeof r !== "function") {
            expandId(r);
        }
    }

    return e;
}
