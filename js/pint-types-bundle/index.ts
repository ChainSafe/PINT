import {
    OverrideBundleDefinition,
    OverrideBundleType,
} from "@polkadot/types/types";

export const definitions = {
    types: [
        {
            // on all versions
            minmax: [0, undefined],
            types: {
                Address: "MultiAddress",
                LookupSource: "MultiAddress",
                AssetId: "u32",
                AccountIdFor: "AccountId",
                Balance: "u128",
                BalanceFor: "Balance",
                FeedId: "u64",
                HashFor: "Hash",
                AssetAvailability: {
                    _enum: {
                        Liquid: "MultiLocation",
                        Saft: null,
                    },
                },
            },
        },
    ],
} as OverrideBundleDefinition;

export const typesBundle = {
    spec: {
        pint: definitions,
    },
} as OverrideBundleType;
