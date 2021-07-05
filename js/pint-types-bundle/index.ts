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
                LookupSourceFor: "LookupSource",
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
                AssetWithdrawal: {
                    asset: "AssetId",
                    state: "RedemptionState",
                    units: "Balance",
                },
                Balance: "u128",
                BalanceFor: "Balance",
                CommitteeMember: {
                    account_id: "AccountId",
                    member_type: "MemberType",
                },
                CurrencyId: "AssetId",
                CurrencyIdOf: "CurrencyId",
                FeedId: "u64",
                FeedIdFor: "FeedId",
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
                PendingRedemption: {
                    initiated: "BlockNumber",
                    assets: "Vec<AssetWithdrawal>",
                },
                ProposalNonce: "u32",
                ProxyType: {
                    _enum: ["Any", "NonTransfer", "Governance", "Staking"],
                },
                ProxyState: {
                    added: "Vec<ProxyType>",
                },
                RedemptionState: {
                    _enum: {
                        Initiated: null,
                        Unbonding: null,
                        Transferred: null,
                    },
                },
                SAFTRecord: {
                    nav: "Balance",
                    units: "Balance",
                },
                StakingBondState: {
                    controller: "LookupSourceFor",
                    bonded: "Balance",
                    unbonded: "Balance",
                    unlocked_chunks: "u32",
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
