/**
 * Runner extensions
 */
import { ISubmittableResult } from "@polkadot/types/types";
import { DispatchError, EventRecord } from "@polkadot/types/interfaces/types";
import { ApiPromise, WsProvider } from "@polkadot/api";
import { Keyring } from "@polkadot/keyring";
import { KeyringPair } from "@polkadot/keyring/types";
import { definitions } from "@pint/types";
import { Config, Extrinsic } from "./config";
import { launch } from "./launch";

// Extrinsics builder
type Builder = (api: ApiPromise) => Extrinsic[];

// Message of launching complete
const LAUNCH_COMPLETE: string = "POLKADOT LAUNCH COMPLETE";

/**
 * E2E runner
 */
export default class Runner implements Config {
    public api: ApiPromise;
    public pair: KeyringPair;
    public exs: Extrinsic[];

    /**
     * run E2E tests
     *
     * @param {Builder} exs - Extrinsic builder
     * @param {string} ws - "ws://0.0.0.0:9988" by default
     * @param {string} uri - "//Alice" by default
     * @returns {Promise<Runner>}
     */
    static async run(
        exs: Builder,
        ws: string = "ws://127.0.0.1:9988",
        uri: string = "//Alice"
    ): Promise<void> {
        console.log("bootstrap e2e tests...");
        console.log("establishing ws connections... (around 2 mins)");
        const ps = await launch("inherit");
        (ps as any).stdout.on("data", async (chunk: string) => {
            console.log(chunk);
            if (chunk.includes(LAUNCH_COMPLETE)) {
                console.log("COMPLETE LAUNCH!");
                const runner = await Runner.build(exs, ws, uri);
                await runner.runTxs();
            }
        });

        // Kill all processes when exiting.
        process.on("exit", async () => {
            ps.send && ps.send("exit");
            process.exit(2);
        });

        // Handle ctrl+c to trigger `exit`.
        process.on("SIGINT", async () => {
            ps.send && ps.send("exit");
            process.exit(2);
        });
    }

    /**
     * Build runner
     *
     * @param {string} ws - "ws://0.0.0.0:9988" by default
     * @param {string} uri - "//Alice" by default
     * @param {Extrinsic[]} exs - extrinsics
     * @returns {Promise<Runner>}
     */
    static async build(
        exs: Builder,
        ws: string = "ws://0.0.0.0:9988",
        uri: string = "//Alice"
    ): Promise<Runner> {
        const provider = new WsProvider(ws);
        const keyring = new Keyring({ type: "sr25519" });
        const pair = keyring.addFromUri(uri);
        const api = await ApiPromise.create({
            provider,
            types: (definitions.types as any)[0].types,
        });

        return new Runner({ api, pair, exs: exs(api) });
    }

    constructor(config: Config) {
        this.api = config.api;
        this.pair = config.pair;
        this.exs = config.exs;
    }

    /**
     * Execute transactions
     *
     * @returns void
     */
    public async runTxs(): Promise<void> {
        this.exs.forEach(async (ex: Extrinsic) => {
            console.log(`run extrinsic ${ex.pallet}.${ex.call}...`);
            console.log(`\t arguments: ${JSON.stringify(ex.args)}`);

            if (ex.block) await this.waitBlock(ex.block);
            await this.timeout(
                this.api.tx[ex.pallet]
                    [ex.call](...ex.args)
                    .signAndSend(this.pair, (res) => {
                        this.checkError(res);
                    }),
                ex.timeout
            ).catch((e) => {
                throw e;
            });
        });

        // exit
        console.log("COMPLETE TESTS!");
        process.exit(0);
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
                                this.api.registry.findMetaError(
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
