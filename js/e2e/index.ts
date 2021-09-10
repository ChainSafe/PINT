/**
 * E2E tests for PINT
 */
import {
    assert,
    Runner,
    expandId,
    Extrinsic,
    ExtrinsicConfig,
    IExtrinsic,
    waitBlock,
} from "./src";
import { ApiPromise } from "@polkadot/api";
import { Balance } from "@polkadot/types/interfaces/runtime";
import BN from "bn.js";

const ASSET_ID_A: number = 42;
const ASSET_ID_A_UNITS: BN = new BN(100);
const ASSET_ID_A_AMOUNT: BN = new BN(100);
const ASSET_ID_B: number = 43;
const ASSET_ID_B_NAV: BN = new BN(100);
const ASSET_ID_B_UNITS: BN = new BN(100);
const BALANCE_THOUSAND: BN = new BN(1000);
const VOTING_PERIOD: number = 10;
const WITHDRAWALS_PERIOD: number = 10;

const TESTS = (api: ApiPromise, config: ExtrinsicConfig): Extrinsic[] => {
    const PINT: Balance = api.createType("Balance", Math.pow(10, 12));
    const PARENT_LOCATION = api.createType("MultiLocation", {
        X2: [
            api.createType("Junction", { Parent: null }),
            api.createType("Junction", {
                AccountId32: {
                    network: "Any",
                    id: config.alice.address,
                },
            }),
        ],
    });

    return [
        /* balance */
        {
            signed: config.alice,
            pallet: "balances",
            call: "transfer",
            args: [config.charlie.address, PINT.mul(BALANCE_THOUSAND)],
            with: [
                {
                    signed: config.alice,
                    pallet: "balances",
                    call: "transfer",
                    args: [config.bob.address, PINT.mul(BALANCE_THOUSAND)],
                },
                {
                    signed: config.alice,
                    pallet: "balances",
                    call: "transfer",
                    args: [config.charlie.address, PINT.mul(BALANCE_THOUSAND)],
                },
                {
                    signed: config.alice,
                    pallet: "balances",
                    call: "transfer",
                    args: [config.dave.address, PINT.mul(BALANCE_THOUSAND)],
                },
            ],
        },
        /* committee */
        {
            pallet: "committee",
            call: "addConstituent",
            args: [config.ziggy.address],
            verify: async () => {
                assert(
                    (
                        (await api.query.committee.members(
                            config.ziggy.address
                        )) as any
                    ).isSome,
                    "Add constituent failed"
                );
            },
        },
        /* local_treasury */
        {
            pallet: "localTreasury",
            call: "withdraw",
            args: [500000000000, config.ziggy.address],
            verify: async () => {
                assert(
                    (
                        await api.derive.balances.all(config.ziggy.address)
                    ).freeBalance.toNumber() === 500000000000,
                    "localTreasury.withdraw failed"
                );
            },
        },
        /* orml_tokens */
        {
            required: ["assetIndex.addAsset"],
            pallet: "tokens",
            call: "setBalance",
            args: [
                config.alice.address,
                ASSET_ID_A,
                PINT.mul(new BN(ASSET_ID_A_UNITS)),
                0,
            ],
        },
        /* chainlink_feed */
        {
            signed: config.alice,
            pallet: "chainlinkFeed",
            call: "createFeed",
            args: [
                PINT.mul(new BN(100)),
                0,
                [1, 100],
                1,
                0,
                "test_feed",
                0,
                [[config.alice.address, config.bob.address]],
                null,
                null,
            ],
            verify: async () => {
                assert(
                    (await api.query.chainlinkFeed.feeds.entries()).length ===
                        1,
                    "Create feed failed"
                );
            },
        },
        {
            required: ["chainlinkFeed.createFeed"],
            signed: config.alice,
            pallet: "chainlinkFeed",
            call: "submit",
            args: [0, 1, 1],
            verify: async () => {
                assert(
                    (await api.query.chainlinkFeed.rounds(0, 1)).isEmpty,
                    "Create feed failed"
                );
            },
        },
        /* price-feed */
        {
            // required: ["votes.assetIndex.addAsset"],
            required: ["assetIndex.addAsset"],
            pallet: "priceFeed",
            call: "mapAssetPriceFeed",
            args: [ASSET_ID_A, 0],
            verify: async () => {
                assert(
                    Number(
                        (
                            await api.query.priceFeed.assetFeeds(ASSET_ID_A)
                        ).toHuman()
                    ) === 0,
                    "map feed failed"
                );
            },
        },
        {
            // required: ["votes.assetIndex.withdraw"],
            required: ["assetIndex.setMetadata"],
            pallet: "priceFeed",
            call: "unmapAssetPriceFeed",
            args: [ASSET_ID_A],
            verify: async () => {
                assert(
                    ((await api.query.priceFeed.assetFeeds(ASSET_ID_A)) as any)
                        .isNone,
                    "unmap price feed failed"
                );
            },
        },
        /* asset-index */
        {
            // proposal: true,
            signed: config.alice,
            pallet: "assetIndex",
            call: "registerAsset",
            args: [
                ASSET_ID_A,
                api.createType("AssetAvailability" as any, {
                    Liquid: PARENT_LOCATION,
                }),
            ],
            verify: async () => {
                assert(
                    ((await api.query.assetIndex.assets(ASSET_ID_A)) as any)
                        .isSome,
                    "assetIndex.addAsset failed"
                );
            },
        },
        {
            // proposal: true,
            signed: config.alice,
            pallet: "assetIndex",
            call: "addAsset",
            shared: async () => {
                return (await api.query.system.account(config.alice.address))
                    .data.free;
            },
            args: [
                ASSET_ID_A,
                PINT.mul(ASSET_ID_A_UNITS),
                PARENT_LOCATION,
                PINT.mul(ASSET_ID_A_AMOUNT),
            ],
            verify: async (before: Balance) => {
                const current = (
                    await api.query.system.account(config.alice.address)
                ).data.free;
                assert(
                    current.sub(before).div(PINT).toNumber() ===
                        ASSET_ID_A_AMOUNT.sub(new BN(1)).toNumber(),
                    "assetIndex.addAsset failed"
                );
            },
        },
        {
            // proposal: true,
            // required: ["votes.priceFeed.mapAssetPriceFeed"],
            required: ["priceFeed.mapAssetPriceFeed"],
            signed: config.alice,
            pallet: "assetIndex",
            call: "deposit",
            args: [ASSET_ID_A, PINT.mul(ASSET_ID_A_UNITS)],
            verify: async () => {
                assert(
                    (
                        (
                            await api.query.assetIndex.deposits(
                                config.alice.address
                            )
                        ).toJSON() as any
                    ).length == 1,
                    "assetIndex.deposit failed"
                );
            },
        },
        {
            // proposal: true,
            // required: ["close.assetIndex.deposit"],
            required: ["assetIndex.deposit"],
            signed: config.alice,
            pallet: "assetIndex",
            call: "withdraw",
            args: [PINT.mul(BALANCE_THOUSAND)],
            verify: async () => {
                assert(
                    (
                        (
                            await api.query.assetIndex.pendingWithdrawals(
                                config.alice.address
                            )
                        ).toHuman() as any
                    ).length === 1,
                    "assetIndex.withdraw failed"
                );
            },
        },
        {
            // proposal: true,
            // required: ["close.assetIndex.withdraw"],
            required: ["assetIndex.withdraw"],
            shared: async () => {
                const currentBlock = (
                    await api.derive.chain.bestNumber()
                ).toNumber();
                const pendingWithdrawls =
                    await api.query.assetIndex.pendingWithdrawals(
                        config.alice.address
                    );

                const end = (pendingWithdrawls as any).toHuman()[0].end_block;
                const needsToWait =
                    end - currentBlock > WITHDRAWALS_PERIOD
                        ? end - currentBlock - WITHDRAWALS_PERIOD
                        : 0;

                await waitBlock(needsToWait);
            },
            signed: config.alice,
            pallet: "assetIndex",
            call: "completeWithdraw",
            args: [],
            verify: async () => {
                assert(
                    (
                        (await api.query.assetIndex.pendingWithdrawals(
                            config.alice.address
                        )) as any
                    ).isNone,
                    "assetIndex.completeWithdraw failed"
                );
            },
        },
        {
            // proposal: true,
            signed: config.alice,
            // required: ["votes.priceFeed.mapAssetPriceFeed"],
            required: ["assetIndex.completeWithdraw"],
            pallet: "assetIndex",
            call: "setMetadata",
            args: [ASSET_ID_A, "PINT_TEST", "P", 9],
            verify: async () => {
                assert(
                    JSON.stringify(
                        (
                            await api.query.assetIndex.metadata(ASSET_ID_A)
                        ).toHuman()
                    ) ===
                        JSON.stringify({
                            name: "PINT_TEST",
                            symbol: "P",
                            decimals: "9",
                        }),
                    "assetIndex.setMetadata failed"
                );
            },
        },
        // TODO: XCM
        //
        // {
        //     // proposal: true,
        //     // required: ["votes.priceFeed.unmapAssetPriceFeed"],
        //     required: ["assetIndex.setMetadata"],
        //     signed: config.alice,
        //     pallet: "assetIndex",
        //     call: "removeAsset",
        //     args: [ASSET_ID_A, BALANCE_THOUSAND, null],
        // },
        /* saft-registry */
        {
            required: ["assetIndex.deposit"],
            signed: config.alice,
            pallet: "saftRegistry",
            call: "addSaft",
            args: [ASSET_ID_B, ASSET_ID_B_NAV, ASSET_ID_B_UNITS],
            verify: async () => {
                assert(
                    ((await api.query.assetIndex.assets(ASSET_ID_B)) as any)
                        .isSome,
                    "Add saft failed"
                );
            },
        },
        {
            required: ["saftRegistry.addSaft"],
            signed: config.alice,
            pallet: "saftRegistry",
            call: "reportNav",
            args: [ASSET_ID_B, 0, ASSET_ID_B_NAV],
            verify: async () => {
                const saft = (
                    (await api.query.saftRegistry.activeSAFTs(
                        ASSET_ID_B,
                        0
                    )) as any
                ).toJSON();
                const expect = {
                    nav: 100,
                    units: 100,
                };

                assert(
                    JSON.stringify(saft[0]) === JSON.stringify(expect),
                    `Report nav failed, expect: ${JSON.stringify(
                        expect
                    )}, result: ${JSON.stringify(saft[0])}`
                );
            },
        },
        /* remote-asset-manager */
        // {
        //     required: ["assetIndex.addAsset"],
        //     signed: config.alice,
        //     pallet: "remoteAssetManager",
        //     call: "sendAddProxy",
        //     args: [ASSET_ID_A, "Any", config.alice.address],
        //     verify: async () => {
        //         assert(
        //             JSON.stringify(
        //                 (
        //                     await api.query.remoteAssetManager.proxies(
        //                         ASSET_ID_A,
        //                         config.alice.address
        //                     )
        //                 ).toJSON()
        //             ) ===
        //                 JSON.stringify({
        //                     added: ["Any"],
        //                 }),
        //             "remoteAssetManager.sendAddProxy failed"
        //         );
        //     },
        // },
        // {
        //     required: ["priceFeed.mapAssetPriceFeed"],
        //     signed: config.alice,
        //     pallet: "remoteAssetManager",
        //     call: "sendBond",
        //     args: [
        //         ASSET_ID_A,
        //         config.alice.address,
        //         1000,
        //         api.createType("RewardDestination", {
        //             Staked: null,
        //         }),
        //     ],
        //     verify: async () => {
        //         assert(
        //             JSON.stringify(
        //                 (
        //                     await api.query.remoteAssetManager.palletStakingLedger(
        //                         ASSET_ID_A
        //                     )
        //                 ).toJSON()
        //             ) ===
        //                 JSON.stringify({
        //                     controller: config.alice.address,
        //                     bonded: 1000,
        //                     unbonded: 0,
        //                     unlocked_chunks: [0],
        //                 }),
        //
        //             "remoteAssetManager.sendBond failed"
        //         );
        //     },
        // },
    ].map((e) => new Extrinsic(expandId(e), api, config.alice));
};

// main
(async () => {
    await Runner.run(TESTS);
})();
