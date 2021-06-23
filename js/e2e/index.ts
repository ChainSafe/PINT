/**
 * E2E tests for PINT
 */
import { Runner, Extrinsic, ExtrinsicConfig } from "./src";
import { ApiPromise } from "@polkadot/api";
import { assert } from "console";

// Tests
const TESTS = (api: ApiPromise, config: ExtrinsicConfig): Extrinsic[] => {
    return [
        /* asset-index */
        // {
        //     signed: true,
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
        //             ((await api.query.assetIndex.holdings(42)) as any).isSome
        //         );
        //     },
        // },
        /* local-treasury */
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
