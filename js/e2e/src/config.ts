/**
 * Config of E2E tests
 */
import { ApiPromise } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";

/**
 * Extrinsic definition
 */
export interface Extrinsic {
    pallet: string;
    call: string;
    args: any[];
    block?: number;
    timeout?: number;
    /// Required finalized calls before this call
    required?: Extrinsic[];
}

/**
 * The config of e2e tests
 */
export interface Config {
    api: ApiPromise;
    pair: KeyringPair;
    exs: Extrinsic[];
}
