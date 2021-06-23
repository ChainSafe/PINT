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
    const ziggy = keyring.addFromUri("//Ziggy");

    return [
        /* balance */
        {
            pallet: "balances",
            call: "transfer",
            args: [ziggy.address, 1000000],
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
        },
        /* committee */
        {
            pallet: "committee",
            call: "propose",
            args: [api.tx.committee.addConstituent(ziggy.address)],
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
        {
            pallet: "saftRegistry",
            call: "reportNav",
            args: [42, 0, 168],
        },
        {
            pallet: "saftRegistry",
            call: "removeSaft",
            args: [42, 0],
        },
    ];
};

// main
(async () => {
    await Runner.run(TESTS);
})();
