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
const ASSET_ID_A_UNITS: number = 1;
const ASSET_ID_A_VALUE: number = 1;
const ASSET_ID_A_DEPOSIT: BN = new BN(10000);
const ASSET_ID_B: number = 43;
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
        /* asset-index */
        {
            proposal: true,
            signed: config.alice,
            required: ["votes.priceFeed.mapAssetPriceFeed"],
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
        {
            proposal: true,
            signed: config.alice,
            pallet: "assetIndex",
            call: "addAsset",
            args: [
                ASSET_ID_A,
                ASSET_ID_A_UNITS,
                PARENT_LOCATION,
                ASSET_ID_A_VALUE,
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
            proposal: true,
            required: ["votes.assetIndex.setMetadata"],
            shared: async () => {
                return (await api.query.system.account(config.alice.address))
                    .data.free;
            },
            signed: config.alice,
            pallet: "assetIndex",
            call: "deposit",
            args: [ASSET_ID_A, PINT.mul(ASSET_ID_A_DEPOSIT)],
            verify: async (_before: Balance) => {
                // const current = (
                //     await api.query.system.account(config.alice.address)
                // ).data.free;
                //
                // // cover weight fee
                // assert(
                //     current.sub(before).div(PINT).toNumber() ===
                //         ASSET_ID_A_DEPOSIT.toNumber() - 1,
                //     "assetIndex.deposit failed"
                // );
            },
        },
        {
            proposal: true,
            required: ["close.assetIndex.deposit"],
            signed: config.alice,
            pallet: "assetIndex",
            call: "withdraw",
            args: [PINT.mul(BALANCE_THOUSAND).div(new BN(4))],
            verify: async () => {
                // assert(
                //     (
                //         (
                //             await api.query.assetIndex.pendingWithdrawals(
                //                 config.alice.address
                //             )
                //         ).toHuman() as any
                //     ).length === 1,
                //     "assetIndex.withdraw failed"
                // );
            },
        },
        // {
        //     required: ["assetIndex.withdraw"],
        //     shared: async () => {
        //         const currentBlock = (
        //             await api.derive.chain.bestNumber()
        //         ).toNumber();
        //         const pendingWithdrawls =
        //             await api.query.assetIndex.pendingWithdrawals(
        //                 config.alice.address
        //             );
        //
        //         const end = (pendingWithdrawls as any).toHuman()[0].end_block;
        //         const needsToWait =
        //             end - currentBlock > WITHDRAWALS_PERIOD
        //                 ? end - currentBlock - WITHDRAWALS_PERIOD
        //                 : 0;
        //
        //         console.log(
        //             `\t | waiting for the withdrawls peirod (around ${Math.floor(
        //                 (needsToWait * 12) / 60
        //             )} mins)...`
        //         );
        //
        //         await waitBlock(needsToWait);
        //     },
        //     signed: config.alice,
        //     pallet: "assetIndex",
        //     call: "completeWithdraw",
        //     args: [],
        //     verify: async () => {
        //         // assert(
        //         //     (
        //         //         (await api.query.assetIndex.pendingWithdrawals(
        //         //             config.alice.address
        //         //         )) as any
        //         //     ).isNone,
        //         //     "assetIndex.completeWithdraw failed"
        //         // );
        //     },
        // },
        /* remote-asset-manager*/
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
        /* chainlink_feed */
        {
            signed: config.alice,
            pallet: "chainlinkFeed",
            call: "createFeed",
            args: [
                100000000000,
                0,
                [100000000000, 100000000000],
                1,
                9,
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
            args: [0, 1, 100000000000],
            verify: async () => {
                assert(
                    (await api.query.chainlinkFeed.rounds(0, 1)).isEmpty,
                    "Create feed failed"
                );
            },
        },
        /* price-feed */
        {
            proposal: true,
            signed: config.alice,
            required: ["votes.assetIndex.addAsset"],
            pallet: "priceFeed",
            call: "mapAssetPriceFeed",
            args: [ASSET_ID_A, 0],
            verify: async () => {
                // assert(
                //     Number(
                //         (
                //             await api.query.priceFeed.assetFeeds(ASSET_ID_A)
                //         ).toHuman()
                //     ) === 0,
                //     "map feed failed"
                // );
            },
        },
        {
            proposal: true,
            required: ["close.assetIndex.withdraw"],
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
        /* saft-registry */
        // {
        //     signed: config.alice,
        //     pallet: "saftRegistry",
        //     call: "addSaft",
        //     args: [ASSET_ID_B, 168, 42],
        //     verify: async () => {
        //         assert(
        //             ((await api.query.assetIndex.assets(ASSET_ID_B)) as any)
        //                 .isSome,
        //             "Add saft failed"
        //         );
        //     },
        // },
        // {
        //     required: ["saftRegistry.addSaft"],
        //     signed: config.alice,
        //     pallet: "saftRegistry",
        //     call: "reportNav",
        //     args: [ASSET_ID_B, 0, 336],
        //     verify: async () => {
        //         const saft = (
        //             (await api.query.saftRegistry.activeSAFTs(
        //                 ASSET_ID_B,
        //                 Number(
        //                     (
        //                         await api.query.saftRegistry.sAFTCounter(
        //                             ASSET_ID_B
        //                         )
        //                     ).toHuman()
        //                 )
        //             )) as any
        //         ).toJSON();
        //         const expect = {
        //             nav: 336,
        //             units: 42,
        //         };
        //         assert(
        //             JSON.stringify(saft[0]) ===
        //                 JSON.stringify({
        //                     nav: 336,
        //                     units: 42,
        //                 }),
        //             `Report nav failed, expect: ${JSON.stringify(
        //                 expect
        //             )}, result: ${JSON.stringify(saft[0])}`
        //         );
        //     },
        // },
        /* asset-index */
        {
            proposal: true,
            required: ["votes.priceFeed.unmapAssetPriceFeed"],
            signed: config.alice,
            pallet: "assetIndex",
            call: "removeAsset",
            args: [ASSET_ID_A, BALANCE_THOUSAND, null],
            verify: async () => {
                assert(
                    ((await api.query.assetIndex.assets(ASSET_ID_A)) as any)
                        .isNone,
                    "assetIndex.removeAsset failed"
                );
            },
        },
    ].map((e) => new Extrinsic(expandId(e), api, config.alice));
};

// main
(async () => {
    await Runner.run(TESTS);
})();
