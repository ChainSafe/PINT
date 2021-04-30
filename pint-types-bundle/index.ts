import {OverrideBundleDefinition, OverrideBundleType,} from "@polkadot/types/types";

export const definitions = {
    types: [
        {
            // on all versions
            minmax: [0, undefined],
            types: {
                AssetId: "u32",
                AccountIdFor: "AccountId",
                Balance: "u128",
                BalanceFor: "Balance",
                FeedId: "u64",
                HashFor: "Hash"
            },
        }
    ],
} as OverrideBundleDefinition;

export const typesBundle = {
    spec: {
        pint: definitions
    },
} as OverrideBundleType;