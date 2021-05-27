import { ApiPromise, WsProvider } from "@polkadot/api";
import { Keyring } from "@polkadot/keyring";
import { KeyringPair } from "@polkadot/keyring/types";

/**
 *  Tester
 */
class Tester {
  api: ApiPromise;
  pair: KeyringPair;

  /**
   * Init API with provided config
   */
  static async init(
    // PINT websocket port
    wsPort: string = "ws://0.0.0.0:9988",
    // Testing account
    uri: string = "//Alice"
  ): Promise<Tester> {
    // init api
    const provider = new WsProvider(wsPort);
    const api = await ApiPromise.create({ provider });

    // init keyring
    const keyring = new Keyring({ type: "sr25519" });
    const pair = keyring.addFromUri(uri);

    return new Tester(api, pair);
  }

  constructor(api: ApiPromise, pair: KeyringPair) {
    this.api = api;
    this.pair = pair;
  }
}
