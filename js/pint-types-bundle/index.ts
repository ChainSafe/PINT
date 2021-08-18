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
                AccountIdFor: "AccountId",
                AccountBalance: {
                    available: "Balance",
                    reserved: "Balance",
                },
                Amount: "i128",
                AssetAvailability: {
                    _enum: {
                        Liquid: "MultiLocation",
                        Saft: null,
                    },
                },
                AssetId: "u32",
                AssetMetadata: {
                    name: "BoundedString",
                    symbol: "BoundedString",
                    decimals: "u8",
                },
                AssetWithdrawal: {
                    asset: "AssetId",
                    state: "RedemptionState",
                    units: "Balance",
                },
                Balance: "u128",
                BalanceFor: "Balance",
                BoundedString: "BoundedVec<u8, 50>",
                CommitteeMember: {
                    account_id: "AccountId",
                    member_type: "MemberType",
                },
                CurrencyId: "AssetId",
                CurrencyIdOf: "CurrencyId",
                FeeRate: {
                    numerator: "u32",
                    denominator: "u32",
                },
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
                OrmlAccountData: {
                    free: "Balance",
                    frozen: "Balance",
                    reserved: "Balance",
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
                ProxyWeights: {
                    add_proxy: "Weight",
                    remove_proxy: "Weight",
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
                StakingWeights: {
                    bond: "Weight",
                    bond_extra: "Weight",
                    unbond: "Weight",
                    withdraw_unbonded: "Weight",
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
