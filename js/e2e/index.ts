/**
 * E2E tests for PINT
 */
import { Runner, Extrinsic, ExtrinsicConfig } from "./src";
import { ApiPromise } from "@polkadot/api";
import { assert } from "console";

const TESTS = (api: ApiPromise, config: ExtrinsicConfig): Extrinsic[] => {
    return [
        /* balance */
        {
            pallet: "balances",
            call: "transfer",
            args: [config.ziggy.address, 1000000],
        },
        /* asset-index */
        {
            signed: config.alice,
            pallet: "assetIndex",
            call: "addAsset",
            args: [
                42,
                1000000,
                api.createType("AssetAvailability" as any),
                1000000,
            ],
            verify: async () => {
                assert(
                    ((await api.query.assetIndex.holdings(42)) as any).isSome,
                    "assetIndex.addAsset failed"
                );
            },
        },
        /* committee */
        {
            signed: config.alice,
            pallet: "committee",
            call: "propose",
            args: [api.tx.balances.transfer(config.ziggy.address, 1000000)],
            verify: async () => {
                const proposals = await api.query.committee.activeProposals();
                assert(
                    (proposals as any).length > 0,
                    "no proposal found after committe.propose"
                );
            },
        },
        {
            signed: config.alice,
            pallet: "committee",
            call: "vote",
            args: [
                async () => {
                    return new Promise(async (resolve) => {
                        const currentBlock: any = await api.query.system.number();
                        while (
                            (await api.query.system.number()) <
                            currentBlock + 14
                        ) {
                            console.log("\t | waiting for voting peirod...");
                            await Runner.waitBlock(15);
                        }

                        const hash = ((await api.query.committee.activeProposals()) as any)[0];
                        resolve(hash);
                    });
                },
                api.createType("Vote" as any),
            ],
            verify: async () => {
                const hash = ((await api.query.committee.activeProposals()) as any)[0];
                assert(
                    ((await api.query.committee.votes(hash)).toJSON() as any)
                        .votes[0]["vote"] == "Aye",
                    "committee.vote failed"
                );
            },
        },
        {
            pallet: "committee",
            call: "close",
            args: [
                async () =>
                    new Promise(async (resolve) => {
                        const currentBlock: any = await api.query.system.number();
                        while (
                            (await api.query.system.number()) >
                            currentBlock + 5
                        ) {
                            console.log(
                                "\t | waiting for the end of voting peirod..."
                            );
                        }
                        resolve(
                            ((await api.query.committee.activeProposals()) as any)[0]
                        );
                    }),
            ],
            required: [
                {
                    signed: config.bob,
                    pallet: "committee",
                    call: "vote",
                    args: [
                        async () =>
                            ((await api.query.committee.activeProposals()) as any)[0],
                        api.createType("Vote" as any),
                    ],
                },
                {
                    signed: config.charlie,
                    pallet: "committee",
                    call: "vote",
                    args: [
                        async () =>
                            ((await api.query.committee.activeProposals()) as any)[0],
                        api.createType("Vote" as any),
                    ],
                },
                {
                    signed: config.dave,
                    pallet: "committee",
                    call: "vote",
                    args: [
                        async () =>
                            ((await api.query.committee.activeProposals()) as any)[0],
                        api.createType("Vote" as any),
                    ],
                },
            ],
            verify: async () => {
                const proposals = await api.query.committee.executedProposals();
                assert(
                    (proposals as any).length > 0,
                    "no proposal executed after committe.close"
                );
            },
        },
        {
            pallet: "committee",
            call: "addConstituent",
            args: [config.ziggy.address],
            verify: async () => {
                assert(
                    ((await api.query.committee.members(
                        config.ziggy.address
                    )) as any).isSome,
                    "Add constituent failed"
                );
            },
        },
        /* local_treasury */
        {
            pallet: "localTreasury",
            call: "withdraw",
            args: [100000000, config.charlie.address],
            verify: async () => {
                // TODO:
                //
                // The result of this call is weird, no value transfered,
                // needs to check the currency config of pallet
                // local_treasury
            },
        },
        /* price-feed */
        {
            pallet: "priceFeed",
            call: "trackAssetPriceFeed",
            args: [42, 0],
        },
        {
            pallet: "priceFeed",
            call: "untrackAssetPriceFeed",
            args: [42],
        },
        /* saft-registry */
        {
            pallet: "saftRegistry",
            call: "addSaft",
            args: [43, 168, 42],
            verify: async () => {
                assert(
                    ((await api.query.assetIndex.holdings(43)) as any).isSome,
                    "Add saft failed"
                );
            },
        },
        {
            pallet: "saftRegistry",
            call: "reportNav",
            args: [43, 0, 336],
            verify: async () => {
                const saft = ((await api.query.saftRegistry.activeSAFTs(
                    43
                )) as any).toJSON();
                assert(
                    ((await api.query.saftRegistry.activeSAFTs(
                        43
                    )) as any)[0].toJSON() ===
                        {
                            nav: 336,
                            units: 42,
                        },
                    "Report nav failed"
                );
            },
        },
        // TODO:
        //
        // requires https://github.com/ChainSafe/PINT/pull/73
        //
        // {
        //     pallet: "saftRegistry",
        //     call: "removeSaft",
        //     args: [42, 0],
        // },
    ];
};

// main
(async () => {
    await Runner.run(TESTS);
})();
