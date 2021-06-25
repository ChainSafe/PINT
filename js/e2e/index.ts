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
            signed: config.alice,
            inBlock: true,
            pallet: "balances",
            call: "transfer",
            args: [config.charlie.address, 10000000000000],
            post: [
                {
                    signed: config.alice,
                    inBlock: true,
                    pallet: "balances",
                    call: "transfer",
                    args: [config.dave.address, 10000000000000],
                },
            ],
        },
        // /* asset-index */
        // {
        //     signed: config.alice,
        //     pallet: "assetIndex",
        //     call: "addAsset",
        //     args: [
        //         42,
        //         1000000,
        //         api.createType("AssetAvailability" as any),
        //         1000000,
        //     ],
        //     verify: async () => {
        //         assert(
        //             ((await api.query.assetIndex.holdings(42)) as any).isSome,
        //             "assetIndex.addAsset failed"
        //         );
        //     },
        // },
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
            shared: async () => {
                await Runner.waitBlock(1);
                const hash = ((await api.query.committee.activeProposals()) as any)[0];

                return hash;
            },
            inBlock: true,
            signed: config.alice,
            pallet: "committee",
            call: "vote",
            args: [
                async (hash: string) => {
                    return new Promise(async (resolve) => {
                        const currentBlock = (
                            await api.derive.chain.bestNumber()
                        ).toNumber();
                        console.log(`\t | current block: ${currentBlock}`);

                        console.log("\t | waiting for the voting peirod...");
                        const end = ((
                            await api.query.committee.votes(hash)
                        ).toJSON() as any).end as number;

                        await Runner.waitBlock(
                            end - currentBlock > 10
                                ? end - currentBlock - 10
                                : 0
                        );
                        console.log(
                            `\t | current block: ${await api.derive.chain.bestNumber()}`
                        );

                        resolve(hash);
                    });
                },
                api.createType("Vote" as any),
            ],
            // Post calls
            post: [
                async (hash: string): Promise<Extrinsic> => {
                    return {
                        inBlock: true,
                        signed: config.bob,
                        pallet: "committee",
                        call: "vote",
                        args: [hash, api.createType("Vote" as any)],
                    };
                },
                async (hash: string): Promise<Extrinsic> => {
                    return {
                        inBlock: true,
                        signed: config.charlie,
                        pallet: "committee",
                        call: "vote",
                        args: [hash, api.createType("Vote" as any)],
                    };
                },
                async (hash: string): Promise<Extrinsic> => {
                    return {
                        signed: config.dave,
                        pallet: "committee",
                        call: "vote",
                        args: [hash, api.createType("Vote" as any)],
                    };
                },
                async (hash: string): Promise<Extrinsic> => {
                    return {
                        signed: config.alice,
                        pallet: "committee",
                        call: "close",
                        args: [hash],
                        verify: async () => {
                            const proposals = await api.query.committee.executedProposals(
                                hash
                            );
                            assert(
                                (proposals as any).isSome,
                                "no proposal executed after committe.close"
                            );
                        },
                    };
                },
            ],
            verify: async (hash: string) => {
                assert(
                    ((await api.query.committee.votes(hash)).toJSON() as any)
                        .votes[0]["vote"] == "Aye",
                    "committee.vote failed"
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
                    saft ===
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
