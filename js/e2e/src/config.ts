/**
 * Config of E2E tests
 */
import { ApiPromise } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";

/**
 * Extrinsic definition
 */
export interface Extrinsic {
    // extrinsic id
    id?: string;
    inBlock?: boolean;
    // use signed origin
    signed?: KeyringPair;
    pallet: string;
    call: string;
    args: any[];
    shared?: () => Promise<any>;
    verify?: (shared?: any) => Promise<void>;
    /// Required calls or functions before this extrinsic
    required?: (string | Extrinsic | ((shared?: any) => Promise<Extrinsic>))[];
    /// Post calls or functions before this extrinsic
    post?: (Extrinsic | ((shared?: any) => Promise<Extrinsic>))[];
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
    alice: KeyringPair;
    bob: KeyringPair;
    charlie: KeyringPair;
    dave: KeyringPair;
    ziggy: KeyringPair;
}
