/**
 * Extrinsic
 */
import { KeyringPair } from "@polkadot/keyring/types";
import { ExtrinsicConfig, IExtrinsic } from "./config";
import { ISubmittableResult } from "@polkadot/types/types";
import { DispatchError, EventRecord } from "@polkadot/types/interfaces/types";
import { SubmittableExtrinsic } from "@polkadot/api/types";
import { ApiPromise } from "@polkadot/api";
import { waitBlock } from "./util";

const VOTING_PERIOD: number = 10;

// Substrate transaction
export type Transaction = SubmittableExtrinsic<"promise", ISubmittableResult>;

// runTx Result
interface TxResult {
    unsub: Promise<() => void>;
    blockHash: string;
}

/**
 * Custom Extrinsic
 */
export class Extrinsic {
    api: ApiPromise;
    // extrinsic id
    id?: string;
    // use signed origin
    signed?: KeyringPair;
    // default origin
    pair?: KeyringPair;
    pallet: string;
    call: string;
    args: any[];
    shared?: () => Promise<any>;
    verify?: (shared?: any) => Promise<void>;
    /// if this call starts with a proposal
    proposal?: boolean;
    /// Required calls or functions before this extrinsic
    required?: string[];
    /// Calls or functions with this extrinsic
    with?: IExtrinsic[];

    constructor(e: IExtrinsic, api: ApiPromise, pair: KeyringPair) {
        this.api = api;
        this.id = e.id;
        this.signed = e.signed;
        this.pallet = e.pallet;
        this.call = e.call;
        this.args = e.args;
        this.shared = e.shared;
        this.verify = e.verify;
        this.pair = pair;
        this.required = e.required;
        this.with = e.with;
        this.proposal = e.proposal;
    }

    /**
     * Build transaction from extrinsic
     *
     * @param {ex} Extrinsic
     * @returns {Transaction}
     */
    public async build(): Promise<Transaction> {
        if (typeof this.shared === "function") {
            this.shared = await this.shared();
        }

        // flush arguments
        const args: any[] = [];
        for (const arg of this.args) {
            if (typeof arg === "function") {
                args.push(arg(this.shared));
            } else {
                args.push(arg);
            }
        }

        // construct tx
        let tx = this.api.tx[this.pallet][this.call](...args);
        if (!this.signed) {
            tx = this.api.tx.sudo.sudo(tx);
        }

        return tx;
    }

    /**
     * Parse transaction errors
     *
     * @param {ISubmittableResult} sr
     * @returns {Promise<T>}
     */
    private async send(
        se: SubmittableExtrinsic<"promise", ISubmittableResult>,
        nonce: number,
        signed = this.pair
    ): Promise<TxResult> {
        return new Promise((resolve, reject) => {
            const unsub: any = se.signAndSend(
                signed,
                { nonce },
                async (sr: ISubmittableResult) =>
                    await this.checkError(unsub, sr, resolve, reject)
            );
        });
    }

    /**
     * Check and throw transaction errors
     *
     * @param {boolean} inblock
     * @param {Promise<() => void>} unsub
     * @param {ISubmittableResult} sr
     * @param {(value: TxResult | PromiseLike<TxResult>) => void} resolve
     * @param {(reason?: any) => void} reject
     */
    private async checkError(
        unsub: Promise<() => void>,
        sr: ISubmittableResult,
        resolve: (value: TxResult | PromiseLike<TxResult>) => void,
        reject: (reason?: any) => void
    ) {
        const status = sr.status;
        const events = sr.events;

        console.log(`\t | - ${this.id} status: ${status.type}`);

        if (status.isInBlock) {
            if (events) {
                events.forEach((value: EventRecord): void => {
                    const maybeError = value.event.data[0];
                    if (maybeError && (maybeError as DispatchError).isModule) {
                        const error = this.api.registry.findMetaError(
                            (
                                value.event.data[0] as DispatchError
                            ).asModule.toU8a()
                        );
                        reject(
                            `${error.section}.${error.method}: ${error.docs}`
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

    /**
     * Get the latest proposal hash
     */
    private async getLastestProposal(): Promise<string> {
        const activeProposals =
            (await this.api.query.committee.activeProposals()) as any;

        // check if proposed
        if (activeProposals.length < 1) {
            await waitBlock(1);
            return this.getLastestProposal();
        }

        return activeProposals[activeProposals.length - 1];
    }

    /**
     * Run extrinsic with proposal
     */
    public async propose(
        proposals: Record<string, string>,
        queue: Extrinsic[],
        config: ExtrinsicConfig
    ): Promise<void | string> {
        const id = this.id ? this.id : `${this.pallet}.${this.call}`;
        queue.push(
            new Extrinsic(
                {
                    id: `propose.${id}`,
                    required: this.required,
                    signed: this.signed,
                    pallet: "committee",
                    call: "propose",
                    args: [this.api.tx[this.pallet][this.call](...this.args)],
                },
                this.api,
                this.pair
            )
        );

        for (const account of ["alice", "bob", "charlie", "dave"]) {
            queue.push(
                new Extrinsic(
                    {
                        required: [`propose.${id}`],
                        id: `votes.${id}.${account}`,
                        shared: async () => {
                            return new Promise(async (resolve) => {
                                const hash = proposals[this.id];
                                const currentBlock = (
                                    await this.api.derive.chain.bestNumber()
                                ).toNumber();

                                const end = (
                                    (
                                        await this.api.query.committee.votes(
                                            hash
                                        )
                                    ).toJSON() as any
                                ).end as number;

                                const needsToWait =
                                    end - currentBlock > VOTING_PERIOD
                                        ? end - currentBlock - VOTING_PERIOD
                                        : 0;

                                console.log(`\t | voting ${id}...`);
                                console.log(
                                    `\t | waiting for the voting peirod (around ${Math.floor(
                                        (needsToWait * 12) / 60
                                    )} mins)...`
                                );

                                await waitBlock(needsToWait);
                                resolve(hash);
                            });
                        },
                        signed: (config as any)[account],
                        pallet: "committee",
                        call: "vote",
                        args: [
                            () => proposals[this.id],
                            this.api.createType("VoteKind" as any),
                        ],
                    },
                    this.api,
                    this.pair
                )
            );
        }

        // push close and verification
        queue.push(
            new Extrinsic(
                {
                    required: [`votes.${id}.dave`],
                    id: `close.${id}`,
                    shared: async () => {
                        const hash = proposals[this.id];
                        const currentBlock = (
                            await this.api.derive.chain.bestNumber()
                        ).toNumber();

                        const end = (
                            (
                                await this.api.query.committee.votes(hash)
                            ).toJSON() as any
                        ).end as number;

                        const needsToWait = end - currentBlock + 1;
                        console.log(
                            `\t | waiting for the end of voting peirod ( ${needsToWait} blocks )`
                        );

                        await waitBlock(needsToWait > 0 ? needsToWait : 1);
                        if (this.shared && typeof this.shared === "function") {
                            return await this.shared();
                        }
                    },
                    signed: config.alice,
                    pallet: "committee",
                    call: "close",
                    args: [() => proposals[this.id]],
                    verify: this.verify,
                },
                this.api,
                this.pair
            )
        );
    }

    /**
     * Run Extrinsic
     *
     * @param {ex} Extrinsic
     */
    public async run(
        proposals: Record<string, any>,
        finished: string[],
        errors: string[],
        nonce: number
    ): Promise<void | string> {
        console.log(`-> queue extrinsic ${nonce}: ${this.id}...`);
        const tx = await this.build().catch((err) => {
            console.log(
                `====> Error: extrinsic build failed ${this.id} failed: ${err}`
            );
            process.exit(1);
        });

        // get res
        const res = (await this.send(tx, nonce, this.signed).catch(
            (err: any) => {
                console.log(`====> Error: ${this.id} failed: ${err}`);
                errors.push(`====> Error: ${this.id} failed: ${err}`);
            }
        )) as TxResult;

        if (res && res.unsub) {
            (await res.unsub)();
        }

        // this execute verify script
        if (this.verify) {
            console.log(`\t | verify: ${this.id}`);
            await this.verify(this.shared).catch((err: any) => {
                console.log(`====> Error: ${this.id} verify failed: ${err}`);
                errors.push(`====> Error: ${this.id} verify failed: ${err}`);
            });
        }

        // push hash if is proposal
        if (this.id.includes("propose.")) {
            proposals[this.id.split("propose.")[1]] =
                await this.getLastestProposal();
        }

        finished.push(this.id);
    }
}
