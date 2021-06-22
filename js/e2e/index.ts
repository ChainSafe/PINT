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
        },
        /* committee */
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
            args: [42, 168, 42],
        },
    ];
};

// main
(async () => {
    await Runner.run(TESTS);
})();
