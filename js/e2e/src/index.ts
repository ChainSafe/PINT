/**
 * E2E tests for PINT
 */
import { Config, Extrinsic } from "./config";
import { waitBlock, timeout, checkError } from "./ext";

/**
 * Traverse all extrinsics
 */
export async function run(config: Config) {
    config.exs.forEach(async (ex: Extrinsic) => {
        if (ex.block) await waitBlock.call(config, ex.block);
        await timeout(
            config.api.tx[ex.pallet]
                [ex.call](...ex.args)
                .signAndSend(config.pair, checkError)
        );
    });
}
