/**
 * Config of E2E tests
 */
import { ApiPromise } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { Extrinsic } from "./extrinsic";

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
    alice: KeyringPair;
    bob: KeyringPair;
    charlie: KeyringPair;
    dave: KeyringPair;
    ziggy: KeyringPair;
}

/**
 * Extrinsic interface
 */
export interface IExtrinsic {
    // extrinsic id
    id?: string;
    // use signed origin
    signed?: KeyringPair;
    pallet: string;
    call: string;
    args: any[];
    shared?: () => Promise<any>;
    verify?: (shared?: any) => Promise<void>;
    wait?: number;
    /// Required calls or functions before this extrinsic
    required?: string[];
    /// Calls or functions with this extrinsic
    with?: (IExtrinsic | ((shared?: any) => Promise<IExtrinsic>))[];
}
