/**
 * E2E tests for PINT
 */
import { Runner, Extrinsic, ExtrinsicConfig } from "./src";
import { ApiPromise } from "@polkadot/api";
import { assert } from "console";

const ASSET_ID_A: number = 42;
const ASSET_ID_B: number = 43;
const BALANCE_THOUSAND: number = 100000000000;
const VOTING_PERIOD: number = 10;

const TESTS = (api: ApiPromise, config: ExtrinsicConfig): Extrinsic[] => {
    return [
        /* balance */
        {
            signed: config.alice,
            inBlock: true,
            pallet: "balances",
            call: "transfer",
            args: [config.charlie.address, BALANCE_THOUSAND],
            post: [
                {
                    signed: config.alice,
                    inBlock: true,
                    pallet: "balances",
                    call: "transfer",
                    args: [config.dave.address, BALANCE_THOUSAND],
                },
            ],
        },
        /* asset-index */
        {
            signed: config.alice,
            pallet: "assetIndex",
            call: "addAsset",
            args: [
                ASSET_ID_A,
                1000000,
                api.createType("AssetAvailability" as any),
                1000000,
            ],
            verify: async () => {
                assert(
                    ((await api.query.assetIndex.assets(ASSET_ID_A)) as any)
                        .isSome,
                    "assetIndex.addAsset failed"
                );
            },
        },
        /* committee */
        {
            signed: config.alice,
            pallet: "committee",
            call: "propose",
            args: [api.tx.balances.transfer(config.bob.address, 1000000)],
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
                            end - currentBlock > VOTING_PERIOD
                                ? end - currentBlock - VOTING_PERIOD
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
        /* chainlink-feed*/
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
        /* price-feed */
        {
            pallet: "priceFeed",
            call: "trackAssetPriceFeed",
            args: [ASSET_ID_A, 0],
            verify: async () => {
                assert(
                    Number(
                        (
                            await api.query.priceFeed.assetFeeds(ASSET_ID_A)
                        ).toHuman()
                    ) === 0,
                    "Create feed failed"
                );
            },
        },
        {
            pallet: "priceFeed",
            call: "untrackAssetPriceFeed",
            args: [ASSET_ID_A],
            verify: async () => {
                assert(
                    ((await api.query.priceFeed.assetFeeds(ASSET_ID_A)) as any)
                        .isNone,
                    "Create feed failed"
                );
            },
        },
        /* saft-registry */
        {
            pallet: "saftRegistry",
            call: "addSaft",
            args: [ASSET_ID_B, 168, 42],
            verify: async () => {
                assert(
                    ((await api.query.assetIndex.assets(ASSET_ID_B)) as any)
                        .isSome,
                    "Add saft failed"
                );
            },
        },
        {
            pallet: "saftRegistry",
            call: "reportNav",
            args: [ASSET_ID_B, 0, 336],
            verify: async () => {
                const saft = ((await api.query.saftRegistry.activeSAFTs(
                    ASSET_ID_B
                )) as any).toJSON();
                const expect = {
                    nav: 336,
                    units: 42,
                };
                assert(
                    JSON.stringify(saft[0]) ==
                        JSON.stringify({
                            nav: 336,
                            units: 42,
                        }),
                    `Report nav failed, expect: ${JSON.stringify(
                        expect
                    )}, result: ${JSON.stringify(saft[0])}`
                );
            },
        },
    ];
};

// main
(async () => {
    await Runner.run(TESTS);
})();
