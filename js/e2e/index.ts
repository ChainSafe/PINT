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
            args: [config.ziggyAddress, 1000000],
        },
        /* asset-index */
        {
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
                    ((await api.query.assetIndex.holdings(42)) as any).isSome
                );
            },
        },
        /* committee */
        {
            pallet: "committee",
            call: "propose",
            args: [api.tx.committee.addConstituent(config.ziggyAddress)],
        },
        {
            pallet: "committee",
            call: "vote",
            args: [
                async () => {
                    const currentBlock: any = await api.query.system.number();
                    while (
                        (await api.query.system.number()) >
                        currentBlock + 15
                    ) {
                        return ((await api.query.committee.activeProposals()) as any)[0];
                    }
                },
                api.createType("Vote" as any),
            ],
        },
        {
            pallet: "committee",
            call: "close",
            args: [
                async () => {
                    const currentBlock: any = await api.query.system.number();
                    while (
                        (await api.query.system.number()) >
                        currentBlock + 10
                    ) {
                        return ((await api.query.committee.activeProposals()) as any)[0];
                    }
                },
            ],
        },
        /* local_treasury */
        {
            pallet: "localTreasury",
            call: "withdraw",
            args: [100000000, config.charlieAddress],
            verify: async () => {
                console.log(
                    config.bobBalance -
                        (
                            await api.derive.balances.all(config.charlieAddress)
                        ).freeBalance.toBigInt()
                );
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
                    ((await api.query.assetIndex.holdings(43)) as any).isSome
                );
            },
        },
        {
            pallet: "saftRegistry",
            call: "reportNav",
            args: [43, 0, 168],
        },
        // - https://github.com/ChainSafe/PINT/pull/73
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
