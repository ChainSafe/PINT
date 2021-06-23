/**
 * Config of E2E tests
 */
import { ApiPromise } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";

/**
 * Extrinsic definition
 */
export interface Extrinsic {
    // use signed origin
    signed?: boolean;
    pallet: string;
    call: string;
    args: any[];
    block?: number;
    timeout?: number;
    verify?: () => Promise<void>;
    /// Required finalized calls or functions before this extrinsic
    required?: (Extrinsic | (() => Promise<Extrinsic>))[];
}

/**
 * The config of e2e tests
 */
export interface Config {
    api: ApiPromise;
    pair: KeyringPair;
    exs: Extrinsic[];
}

/**
 * The config of extrinsics
 */
export interface ExtrinsicConfig {
    bobAddress: string;
    bobBalance: bigint;
    charlieAddress: string;
    ziggyAddress: string;
}
