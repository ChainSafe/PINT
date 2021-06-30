/**
 * Runner extensions
 */
import { ISubmittableResult } from "@polkadot/types/types";
import { DispatchError, EventRecord } from "@polkadot/types/interfaces/types";
import { ApiPromise, WsProvider } from "@polkadot/api";
import { Keyring } from "@polkadot/keyring";
import { KeyringPair } from "@polkadot/keyring/types";
import ChainlinkTypes from "@pint/types/chainlink.json";
import { definitions } from "@pint/types";
import { Config, Extrinsic, ExtrinsicConfig } from "./config";
import { launch } from "./launch";
import { ChildProcess } from "child_process";
import { cryptoWaitReady } from "@polkadot/util-crypto";
import { SubmittableExtrinsic } from "@polkadot/api/types";
import OrmlTypes from "@open-web3/orml-types";

// Extrinsics builder
type Builder = (api: ApiPromise, config: ExtrinsicConfig) => Extrinsic[];

// runTx Result
interface TxResult {
    unsub: Promise<() => void>;
    blockHash: string;
}

// Message of launching complete
export const LAUNCH_COMPLETE: string = "POLKADOT LAUNCH COMPLETE";

// Kill subprocesses
function killAll(ps: ChildProcess, exitCode: number) {
    try {
        ps.send && !ps.killed && ps.send("exit");
        ps.kill("SIGINT");
    } catch (e) {
        if (e.code !== "EPERM") {
            process.stdout.write(e);
            process.exit(2);
        }
    }

    process.exit(exitCode);
}

/**
 * E2E runner
 */
export default class Runner implements Config {
    public api: ApiPromise;
    public pair: KeyringPair;
    public exs: Extrinsic[];

    /**
     * Wait for n blocks
     *
     * The current gap of producing a block is 4s,
     * we use 5s here.
     *
     * @param {number} block
     * @returns {Promise<void>}
     */
    static async waitBlock(block: number): Promise<void> {
        return new Promise((resolve) => setTimeout(resolve, block * 12000));
    }

    /**
     * run E2E tests without launch
     *
     * @param {Builder} exs - Extrinsic builder
     * @param {string} ws - "ws://0.0.0.0:9988" by default
     * @param {string} uri - "//Alice" by default
     * @returns {Promise<Runner>}
     */
    static async run_without_launch(
        exs: Builder,
        ws: string = "ws://127.0.0.1:9988",
        uri: string = "//Alice"
    ): Promise<void> {
        console.log("bootstrap e2e tests...");
        await cryptoWaitReady();
        const runner = await Runner.build(exs, ws, uri);
        await runner.runTxs();
    }

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
        const ps = await launch("pipe");
        ps.stdout.on("data", async (chunk: Buffer) => {
            process.stdout.write(chunk.toString());
            if (chunk.includes(LAUNCH_COMPLETE)) {
                console.log("COMPLETE LAUNCH!");
                const runner = await Runner.build(exs, ws, uri);
                await runner.runTxs();
            }
        });

        // Log errors
        ps.stderr.on("data", (chunk: Buffer) => console.log(chunk.toString()));

        // Kill all processes when exiting.
        process.on("exit", () => {
            console.log("-> exit polkadot-launch...");
            killAll(ps, process.exitCode);
        });

        // Handle ctrl+c to trigger `exit`.
        process.on("SIGINT", () => killAll(ps, 0));
    }

    /**
     * Build runner
     *
     * @param {string} ws - "ws://127.0.0.1:9988" by default
     * @param {string} uri - "//Alice" by default
     * @param {Extrinsic[]} exs - extrinsics
     * @returns {Promise<Runner>}
     */
    static async build(
        exs: Builder,
        ws: string = "ws://127.0.0.1:9988",
        uri: string = "//Alice"
    ): Promise<Runner> {
        const provider = new WsProvider(ws);
        const keyring = new Keyring({ type: "sr25519" });

        // pairs
        const pair = keyring.addFromUri(uri);
        const alice = keyring.addFromUri("//Alice");
        const bob = keyring.addFromUri("//Bob");
        const charlie = keyring.addFromUri("//Charlie");
        const dave = keyring.addFromUri("//Dave");
        const ziggy = keyring.addFromUri("//Ziggy");

        // create api
        const api = await ApiPromise.create({
            provider,
            types: Object.assign(
                {
                    ...ChainlinkTypes,
                    ...OrmlTypes,
                },
                (definitions.types as any)[0].types
            ),
        });

        // new Runner
        return new Runner({
            api,
            pair,
            exs: exs(api, {
                alice,
                bob,
                charlie,
                dave,
                ziggy,
            }),
        });
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
        for (const ex of this.exs) {
            if (typeof ex.shared === "function") {
                ex.shared = await ex.shared();
            }

            if (ex.required) {
                for (const required of ex.required) {
                    let requiredEx: Extrinsic = required as Extrinsic;
                    if (typeof required === "function") {
                        requiredEx = await required(ex.shared);
                    }

                    console.log(
                        `----> run required extrinsic ${requiredEx.pallet}.${requiredEx.call}...`
                    );
                    await this.runTx(requiredEx);
                }
            }

            console.log(`-> run extrinsic ${ex.pallet}.${ex.call}...`);
            await this.runTx(ex);
        }

        // exit
        console.log("COMPLETE TESTS!");
        process.exit(0);
    }

    /**
     * Run Extrinsic
     *
     * @param {ex} Extrinsic
     */
    public async runTx(ex: Extrinsic): Promise<void> {
        // flush arguments
        const args: any[] = [];
        for (const arg of ex.args) {
            if (typeof arg === "function") {
                args.push(await arg(ex.shared));
            } else {
                args.push(arg);
            }
        }
        console.log(`\t | arguments: ${JSON.stringify(args)}`);

        if (ex.block) await Runner.waitBlock(ex.block);

        // construct tx
        let tx = this.api.tx[ex.pallet][ex.call](...args);
        if (!ex.signed) {
            tx = this.api.tx.sudo.sudo(tx);
        }

        // get res
        const res = (await this.timeout(
            this.sendTx(tx, ex.signed, ex.inBlock),
            ex.timeout
        )) as TxResult;

        // run post calls
        if (ex.post) {
            for (const post of ex.post) {
                let postEx: Extrinsic = post as Extrinsic;
                if (typeof post === "function") {
                    postEx = await post(ex.shared);
                }

                console.log(
                    `----> run post extrinsic ${postEx.pallet}.${postEx.call}...`
                );
                await this.runTx(postEx);
            }
        }

        // execute verify script
        if (ex.verify)
            await ex.verify(ex.shared).catch((err) => {
                throw err;
            });

        (await res.unsub)();
        console.log(`\t | block hash: ${res.blockHash}`);
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
    private async sendTx(
        se: SubmittableExtrinsic<"promise", ISubmittableResult>,
        signed = this.pair,
        inBlock = false
    ): Promise<TxResult> {
        return new Promise((resolve, reject) => {
            const unsub = se.signAndSend(
                signed,
                {},
                (sr: ISubmittableResult) => {
                    const status = sr.status;
                    const events = sr.events;

                    console.log(`\t | - status: ${status.type}`);

                    if (status.isInBlock) {
                        if (inBlock)
                            resolve({
                                unsub,
                                blockHash: status.asInBlock.toHex().toString(),
                            });

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

                                if (
                                    (value.event.data[0] as DispatchError)
                                        .isModule
                                ) {
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
                        resolve({
                            unsub,
                            blockHash: status.asFinalized.toHex().toString(),
                        });
                    }
                }
            );
        });
    }
}
