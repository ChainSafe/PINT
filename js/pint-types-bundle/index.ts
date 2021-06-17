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
                Action: "Call",
                AssetId: "u32",
                AccountIdFor: "AccountId",
                AccountBalance: {
                    available: "Balance",
                    reserved: "Balance",
                },
                AssetAvailability: {
                    _enum: {
                        Liquid: "MultiLocation",
                        Saft: null,
                    },
                },
                Balance: "u128",
                BalanceFor: "Balance",
                CommitteeMember: {
                    account_id: "AccountId",
                    member_type: "MemberType",
                },
                FeedId: "u64",
                HashFor: "Hash",
                IndexAssetData: {
                    units: "Balance",
                    availability: "AssetAvailability",
                },
                MemberType: {
                    _enum: {
                        Council: null,
                        Constituent: null,
                    },
                },
                MemberVote: {
                    member: "CommitteeMember",
                    vote: "Vote",
                },
                Vote: {
                    _enum: {
                        Aye: null,
                        Nay: null,
                        Abstain: null,
                    },
                },
                VoteAggregate: {
                    votes: "Vec<MemberVote>",
                    end: "BlockNumber",
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
