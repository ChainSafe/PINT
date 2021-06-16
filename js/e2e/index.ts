/**
 * E2E tests for PINT
 */
import { Runner, Extrinsic } from "./src";
import { ApiPromise } from "@polkadot/api";

// Tests
const TESTS = (api: ApiPromise): Extrinsic[] => [
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
];

// main
(async () => {
    await Runner.run(TESTS);
})();
