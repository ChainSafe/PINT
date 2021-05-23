// Run pint binary
import path from "path";
import fs from "fs";

/**
 * PINT process
 *
 * ENV ARGS (default "--dev --ws-port 3055")
 * ENV PINT (default "../target/release/pint")
 * ENV WS_PORT (default 3055)
 */
class PINT {
  private _args: string[];
  private path: string;
  private wsPort: number;

  /**
   * Check init PINT binary
   */
  constructor(
    defaultPath: string = path.resolve(__filename, "../target/release/pint"),
    defaultWsPort: number = 3055,
    defaultArgs: string[] = ["--dev"]
  ) {
    const _path = process.env.PINT ? process.env.PINT : defaultPath;
    const _ws_port =
      process.env.WS_PORT && Number(process.env.PINT) !== NaN
        ? Number(process.env.PINT)
        : defaultWsPort;

    // Check if binary exists
    try {
      if (fs.existsSync(_path)) {
        this.path = _path;
        this.wsPort = _ws_port;
      }
    } catch (err) {
      console.error(err);
      process.exit(1);
    }
  }

  /**
   * Build arguments
   */
  private build() {}

  /**
   * Reset arguments, run PINT binary as your wish.
   */
  public args(args: string[]): PINT {
    this._args = args;
    return this;
  }
}
