/* exports */
import { Result } from "../result";
import { Api } from "../api";

/**
 * Call
 */
export type Call = (api: Api, ...args: any[]) => Result<null>;

/**
 * Pallet
 */
export interface Pallet {
    [index: string]: Call;
}

export default {};
