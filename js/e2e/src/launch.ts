/**
 * pint-launch
 */
import fs from "fs";
import findUp from "find-up";
import path from "path";
import { fork, ChildProcess, StdioOptions, spawn } from "child_process";

/**
 * Launch PINT locally
 *
 * @param stdio {StdioOptions}
 * @returns {Promise<ChildProcess>}
 */
export async function local(stdio?: StdioOptions): Promise<ChildProcess> {
    return fork("polkadot-launch", ["config.json"], {
        cwd: path.resolve(String(await findUp("Cargo.toml")), ".."),
        killSignal: "SIGINT",
        stdio,
    } as any);
}

/**
 * Launch PINT via docker (CI)
 *
 * @param stdio {StdioOptions}
 * @returns {Promise<ChildProcess>}
 */
export async function docker(stdio?: StdioOptions): Promise<ChildProcess> {
    return spawn(
        "sudo",
        [
            "docker",
            "run",
            "-p",
            "9988:9988",
            "-p",
            "9966:9966",
            "-p",
            "9999:9999",
            "--ip",
            "0.0.0.0",
            "launch",
        ],
        {
            stdio,
        }
    );
}

/**
 * Launch PINT via local or docker
 *
 * @param stdio {StdioOptions}
 * @returns {Promise<ChildProcess>}
 */
export async function launch(stdio?: StdioOptions): Promise<ChildProcess> {
    const root = await findUp("Cargo.toml");
    if (fs.existsSync(path.resolve(String(root), "../bin/pint"))) {
        return local(stdio);
    } else {
        return docker(stdio);
    }
}
