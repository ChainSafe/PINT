/**
 * pint-launch
 */
import findUp from "find-up";
import path from "path";
import { fork, ChildProcess, StdioOptions, spawn } from "child_process";

/**
 * Launch PINT locally
 */
export async function local(stdio?: StdioOptions): Promise<ChildProcess> {
    return fork("js/polkadot-launch", ["config.json"], {
        cwd: path.resolve(await findUp("Cargo.toml"), ".."),
        stdio,
    });
}

/**
 * Launch PINT via docker (CI)
 */
export async function docker(stdio?: StdioOptions): Promise<ChildProcess> {
    return spawn("docker", ["run", "-it", "launch"], {
        stdio,
    });
}
