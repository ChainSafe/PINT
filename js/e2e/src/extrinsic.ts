/**
 * Extrinsic
 */
import { KeyringPair } from "@polkadot/keyring/types";
import { IExtrinsic } from "./config";
import { ISubmittableResult } from "@polkadot/types/types";
import { DispatchError, EventRecord } from "@polkadot/types/interfaces/types";
import { SubmittableExtrinsic } from "@polkadot/api/types";
import { ApiPromise } from "@polkadot/api";

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
    pair: KeyringPair;
    // extrinsic id
    id?: string;
    // use signed origin
    signed?: KeyringPair;
    pallet: string;
    call: string;
    args: any[];
    shared?: () => Promise<any>;
    verify?: (shared?: any) => Promise<void>;
    /// Required calls or functions before this extrinsic
    required?: string[];
    /// Calls or functions with this extrinsic
    with?: (IExtrinsic | ((shared?: any) => Promise<IExtrinsic>))[];

    constructor(e: IExtrinsic, api: ApiPromise, pair: KeyringPair) {
        this.api = api;
        this.pair = pair;
        this.id = e.id;
        this.signed = e.signed;
        this.pallet = e.pallet;
        this.call = e.call;
        this.args = e.args;
        this.shared = e.shared;
        this.verify = e.verify;
        this.required = e.required;
        this.with = e.with;
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
                            (value.event
                                .data[0] as DispatchError).asModule.toU8a()
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
            await this.verify(this.shared);
        }

        if (res && res.unsub) {
            (await res.unsub)();
        }
    }
}