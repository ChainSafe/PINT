/**
 * Runner extensions
 */
import { ApiPromise, WsProvider } from "@polkadot/api";
import { Keyring } from "@polkadot/keyring";
import { KeyringPair } from "@polkadot/keyring/types";
import ChainlinkTypes from "@pint/types/chainlink.json";
import { definitions } from "@pint/types";
import { Config, ExtrinsicConfig } from "./config";
import { Extrinsic } from "./extrinsic";
import { launch } from "./launch";
import { expandId } from "./util";
import { ChildProcess } from "child_process";
import OrmlTypes from "@open-web3/orml-types";

// Extrinsics builder
type Builder = (api: ApiPromise, config: ExtrinsicConfig) => Extrinsic[];

// Message of launching complete
export const LAUNCH_COMPLETE: string = "POLKADOT LAUNCH COMPLETE";

// Kill subprocesses
function killAll(ps: ChildProcess, exitCode: number) {
    try {
        if (ps.send && !ps.killed) {
            ps.send("exit");
        }
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
    public errors: string[];
    public finished: string[];
    public nonce: number;

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
        if (ps.stdout) {
            ps.stdout.on("data", async (chunk: Buffer) => {
                process.stdout.write(chunk.toString());
                if (chunk.includes(LAUNCH_COMPLETE)) {
                    console.log("COMPLETE LAUNCH!");
                    const runner = await Runner.build(exs, ws, uri);
                    await runner.runTxs();
                }
            });
        }

        // Log errors
        if (ps.stderr) {
            ps.stderr.on("data", (chunk: Buffer) =>
                process.stderr.write(chunk.toString())
            );
        }

        // Kill all processes when exiting.
        process.on("exit", () => {
            console.log("-> exit polkadot-launch...");
            killAll(ps, Number(process.exitCode));
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
        const keyring = new Keyring({ type: "sr25519", ss58Format: 0 });

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
            typesAlias: {
                tokens: {
                    AccountData: "OrmlAccountData",
                    BalanceLock: "OrmlBalanceLock",
                },
            },
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
        this.errors = [];
        this.nonce = 0;
        this.finished = [];
    }

    /**
     * Execute transactions
     *
     * @returns void
     */
    public async runTxs(): Promise<void> {
        while (this.exs.length > 0) {
            await this.queue().catch(console.error);
        }

        if (this.errors.length > 0) {
            console.log(`Failed tests: ${this.errors.length}`);
            for (const error of this.errors) {
                console.log(error);
            }
            process.exit(1);
        }
        console.log("COMPLETE TESTS!");
        process.exit(0);
    }

    /**
     * queue transactions
     *
     * @returns {Promise<void>}
     */
    public async queue(): Promise<void> {
        const runner = this;
        const queue: Extrinsic[] = [];
        for (const e of this.exs) {
            // 0. check if required ex with ids has finished
            let requiredFinished = true;
            if (e.required) {
                for (const r of e.required) {
                    if (this.exs.map((i) => i.id).includes(r)) {
                        requiredFinished = false;
                        break;
                    }
                }
            }

            if (!requiredFinished) {
                continue;
            }

            // 1. Build shared data
            if (typeof e.shared === "function") {
                e.shared = await e.shared.call(this);
            }

            // 2. Pend transactions
            queue.push(e);
            if (e.with) {
                for (const w of e.with) {
                    const ex =
                        typeof w === "function"
                            ? expandId(await w(e.shared))
                            : w;
                    queue.push(new Extrinsic(ex, this.api, this.pair));
                }
            }
        }

        // 3. register transactions
        await this.batch(queue);

        // 4. drop executed exs
        this.exs = this.exs.filter((e) => !queue.includes(e));

        // 5. log pending txs
        for (const ex of this.exs) {
            console.log(`--> ${ex.id} is pending...`);
        }
    }

    /**
     * Batch extrinsics
     */
    public async batch(exs: Extrinsic[]): Promise<any> {
        let currentNonce = Number(this.nonce);
        return Promise.all(
            exs.map((e, i) => {
                let n = -1;
                if (!e.signed || e.signed.address === this.pair.address) {
                    n = Number(currentNonce);
                    currentNonce += 1;
                }
                return e.run(this.errors, n);
            })
        ).then(() => {
            this.nonce = currentNonce;
        });
    }
}
