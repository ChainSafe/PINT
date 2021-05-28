/**
 * Runner extensions
 */
import { Config, Extrinsic } from "./config";
import { ISubmittableResult } from "@polkadot/types/types";
import { DispatchError, EventRecord } from "@polkadot/types/interfaces/types";

/**
 * E2E runner
 */
export default class Runner {
    private config: Config;

    constructor(config: Config) {
        this.config = config;
    }

    /**
     * Run e2e tests
     */
    public run() {
        this.config.exs.forEach(async (ex: Extrinsic) => {
            console.log(`run extrinsic ${ex.pallet}.${ex.call}...`);
            console.log(`\t arguments: ${JSON.stringify(ex.args)}`);

            if (ex.block) await this.waitBlock(ex.block);
            await this.timeout(
                this.config.api.tx[ex.pallet]
                    [ex.call](...ex.args)
                    .signAndSend(this.config.pair, (res) => {
                        this.checkError(res);
                    }),
                ex.timeout
            ).catch((e) => {
                throw e;
            });
        });
    }

    /**
     * Wait for n blocks
     *
     * The current gap of producing a block is 4s,
     * we use 5s here.
     *
     * @param {number} block
     * @returns {Promise<void>}
     */
    private async waitBlock(block: number): Promise<void> {
        return new Promise((resolve) => setTimeout(resolve, block * 5000));
    }

    /**
     * Timeout for promise
     *
     * @param {Promise<T>} fn
     * @param {number} ms
     * @returns {Promise<T>}
     */
    private async timeout<T>(
        fn: Promise<T>,
        ms?: number
    ): Promise<T | unknown> {
        if (!ms) {
            return fn;
        }

        return Promise.race([
            fn,
            new Promise((_, reject) => {
                setTimeout(() => reject("Extrinsic timeout"), ms);
            }),
        ]);
    }

    /**
     * Parse transaction errors
     *
     * @param {ISubmittableResult} sr
     * @returns {Promise<T>}
     */
    private async checkError(sr: ISubmittableResult): Promise<string> {
        return new Promise((resolve, reject) => {
            const status = sr.status;
            const events = sr.events;

            const blockHash = status.asInBlock.toHex().toString();
            if (status.isInBlock) {
                resolve(blockHash);

                if (events) {
                    events.forEach((value: EventRecord): void => {
                        if (value.event.method.indexOf("Failed") > -1) {
                            reject(
                                value.phase.toString() +
                                    `: ${value.event.section}.${value.event.method}` +
                                    value.event.data.toString() +
                                    " failed"
                            );
                        }

                        if ((value.event.data[0] as DispatchError).isModule) {
                            reject(
                                this.config.api.registry.findMetaError(
                                    (value.event
                                        .data[0] as DispatchError).asModule.toU8a()
                                )
                            );
                        }
                    });
                }
            } else if (status.isInvalid) {
                reject("Invalid Extrinsic");
            } else if (status.isRetracted) {
                reject("Extrinsic Retracted");
            } else if (status.isUsurped) {
                reject("Extrinsic Usupred");
            } else if (status.isFinalized) {
                resolve(`Finalized block hash: ${blockHash}`);
            }
        });
    }
}
