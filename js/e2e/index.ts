/**
 * E2E tests for PINT
 */
import { Runner, Extrinsic } from "./src";
import { ApiPromise } from "@polkadot/api";
import { Keyring } from "@polkadot/keyring";

// Tests
const TESTS = (api: ApiPromise): Extrinsic[] => {
    const keyring = new Keyring({ type: "sr25519" });
    const bob = keyring.addFromUri("//Bob");

    return [
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
                console.log(
                    ((await api.query.assetIndex.holdings(42)) as any).isNone
                );
                // if (((await api.query.assetIndex.holdings(42)) as any).isNone) {
                //     throw "The expected asset has not been inserted into storage";
                // }
            },
        },
        /* local-treasury */
        {
            pallet: "localTreasury",
            call: "withdraw",
            args: [42, bob.address],
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
        },
        {
            pallet: "saftRegistry",
            call: "reportNav",
            args: [43, 0, 168],
        },
        // TODO:
        //
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
