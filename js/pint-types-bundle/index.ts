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
                AssetConfig: {
                    pallet_index: "u8",
                    weights: "AssetsWeights",
                },
                AssetId: "u32",
                AssetsWeights: {
                    mint: "Weight",
                    burn: "Weight",
                    transfer: "Weight",
                    force_transfer: "Weight",
                    freeze: "Weight",
                    thaw: "Weight",
                    freeze_asset: "Weight",
                    thaw_asset: "Weight",
                    approve_transfer: "Weight",
                    cancel_approval: "Weight",
                    transfer_approved: "Weight",
                },
                AssetMetadata: {
                    name: "BoundedString",
                    symbol: "BoundedString",
                    decimals: "u8",
                },
                AssetWithdrawal: {
                    asset: "AssetId",
                    reserved: "Balance",
                    units: "Balance",
                    withdrawn: "bool",
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
                DepositRange: {
                    minimum: "Balance",
                    maximum: "Balance",
                },
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
                    end_block: "BlockNumber",
                    assets: "Vec<AssetWithdrawal>",
                },
                Proposal: {
                    nonce: "ProposalNonce",
                    action: "Call",
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
                SAFTId: "u32",
                SAFTRecord: {
                    nav: "Balance",
                    units: "Balance",
                },
                StakingLedger: {
                    controller: "LookupSourceFor",
                    active: "Balance",
                    total: "Balance",
                    unlocking: "Vec<UnlockChunk>",
                },
                StakingLedgerFor: "StakingLedger",
                StakingWeights: {
                    bond: "Weight",
                    bond_extra: "Weight",
                    unbond: "Weight",
                    withdraw_unbonded: "Weight",
                },
                StatemintConfig: {
                    parachain_id: "u32",
                    enabled: "bool",
                    pint_asset_id: "AssetId",
                },
                UnlockChunk: {
                    value: "Balance",
                    end: "BlockNumber",
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
