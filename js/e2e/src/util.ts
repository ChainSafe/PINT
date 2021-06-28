/**
 * Utils
 */

export function assert(r: boolean, msg: string) {
    if (!r) {
        console.error(msg);
        process.exit(1);
    }
}
