/**
 * Utils
 */

export function assert(r: boolean, msg: string): string | void {
    if (!r) {
        return msg;
    }
}
