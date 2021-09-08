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
    with?: (IExtrinsic | ((shared?: any) => Promise<IExtrinsic>))[];

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
    public build(): Transaction {
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
                            `${error.section}.${error.method}: ${error.documentation}`
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
     * Run extrinsic with proposal
     */
    public async propose(
        errors: string[],
        nonce: number,
        queue: Extrinsic[],
        config: ExtrinsicConfig
    ): Promise<void | string> {
        const proposal = new Extrinsic(
            {
                id: `propose.${this.pallet}.${this.call}`,
                signed: this.signed,
                pallet: "committee",
                call: "propose",
                args: [this.api.tx[this.pallet][this.call](...this.args)],
            },
            this.api,
            this.pair
        );

        // propose extrinsic
        await proposal.run(errors, nonce);

        // get the proposal hash
        const proposals =
            (await this.api.query.committee.activeProposals()) as any;

        // check if proposed
        if (!proposals || proposals.length < 1) {
            errors.push(
                `====> Error: ${this.pallet}.${this.call} failed: propose failed`
            );
        }

        // get the proposal hash
        const hash = proposals[proposals.length - 1];
        queue.push(
            new Extrinsic(
                {
                    id: `votes.${this.pallet}.${this.call}`,
                    shared: async () => {
                        return new Promise(async (resolve) => {
                            await waitBlock(1);
                            const currentBlock = (
                                await this.api.derive.chain.bestNumber()
                            ).toNumber();

                            const end = (
                                (
                                    await this.api.query.committee.votes(hash)
                                ).toJSON() as any
                            ).end as number;

                            const needsToWait =
                                end - currentBlock > VOTING_PERIOD
                                    ? end - currentBlock - VOTING_PERIOD
                                    : 0;

                            console.log(
                                `\t | voting ${this.pallet}.${this.call}}`
                            );
                            console.log(
                                `\t | waiting for the voting peirod (around ${Math.floor(
                                    (needsToWait * 12) / 60
                                )} mins)...`
                            );

                            await waitBlock(needsToWait);
                            resolve(hash);
                        });
                    },
                    signed: config.alice,
                    pallet: "committee",
                    call: "vote",
                    args: [
                        (hash: string) => hash,
                        this.api.createType("Vote" as any),
                    ],
                    with: [
                        async (hash: string): Promise<IExtrinsic> => {
                            return {
                                signed: config.bob,
                                pallet: "committee",
                                call: "vote",
                                args: [
                                    hash,
                                    this.api.createType("Vote" as any),
                                ],
                            };
                        },
                        async (hash: string): Promise<IExtrinsic> => {
                            return {
                                signed: config.charlie,
                                pallet: "committee",
                                call: "vote",
                                args: [
                                    hash,
                                    this.api.createType("Vote" as any),
                                ],
                            };
                        },
                        async (hash: string): Promise<IExtrinsic> => {
                            return {
                                signed: config.dave,
                                pallet: "committee",
                                call: "vote",
                                args: [
                                    hash,
                                    this.api.createType("Vote" as any),
                                ],
                            };
                        },
                    ],
                },
                this.api,
                this.pair
            )
        );

        // push close and verification
        queue.push(
            new Extrinsic(
                {
                    required: [`votes.${this.pallet}.${this.call}`],
                    id: `close.${this.pallet}.${this.call}`,
                    shared: async () => {
                        return new Promise(async (resolve) => {
                            const hash = (
                                (await this.api.query.committee.activeProposals()) as any
                            )[0];
                            const currentBlock = (
                                await this.api.derive.chain.bestNumber()
                            ).toNumber();

                            const end = (
                                (
                                    await this.api.query.committee.votes(hash)
                                ).toJSON() as any
                            ).end as number;

                            const needsToWait = end - currentBlock;
                            console.log(
                                `\t | waiting for the end of voting peirod ( ${needsToWait} blocks )`
                            );

                            await waitBlock(needsToWait > 0 ? needsToWait : 0);
                            resolve(hash);
                        });
                    },
                    signed: config.alice,
                    pallet: "committee",
                    call: "close",
                    args: [(hash: string) => hash],
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
    public async run(errors: string[], nonce: number): Promise<void | string> {
        console.log(
            `-> queue extrinsic ${nonce}: ${this.pallet}.${this.call}...`
        );
        const tx = this.build();

        // get res
        const res = (await this.send(tx, nonce, this.signed).catch(
            (err: any) => {
                errors.push(
                    `====> Error: ${this.pallet}.${this.call} failed: ${err}`
                );
            }
        )) as TxResult;

        // thisecute verify script
        if (this.verify) {
            console.log(`\t | verify: ${this.pallet}.${this.call}`);
            await this.verify(this.shared).catch((err: any) => {
                errors.push(
                    `====> Error: ${this.pallet}.${this.call} verify failed: ${err}`
                );
            });
        }

        if (res && res.unsub) {
            (await res.unsub)();
        }
    }
}
