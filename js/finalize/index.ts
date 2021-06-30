/**
 * Test the finalization of parachain intergration
 */
import fs from "fs";
import path from "path";
import { Launch } from "@pint/e2e/src";
import findUp from "find-up";
import { ChildProcess, spawn } from "child_process";

// Message of launching complete
const LAUNCH_COMPLETE: string = "POLKADOT LAUNCH COMPLETE";

// PINT finalization regex
const PINT_FINALIZE: RegExp = /\[Parachain\].*finalized #(\d)/;

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
 * Tail file and done when got expected message
 */
async function tail(
    file: string,
    match: (s: string) => boolean
): Promise<void> {
    const root = await findUp("Cargo.toml");

    return new Promise(async (resolve) => {
        const ps = fs.existsSync(path.resolve(String(root), "../bin/pint"))
            ? spawn("tail", ["-f", file], {
                  cwd: path.resolve(String(root), ".."),
                  stdio: "pipe",
              })
            : spawn("docker", ["exec", "-it", "launch", `tail -f ${file}`], {
                  stdio: "pipe",
              });

        ps.stdout.on("data", (chunk: Buffer) => {
            chunk && match(chunk.toString()) && resolve(null);
        });

        ps.stderr.on("data", (chunk: Buffer) => {
            process.stderr.write(chunk);
            process.exit(1);
        });
    });
}

/**
 * Entrypoint
 */
async function main() {
    const ps = await Launch.launch("pipe");
    ps.stdout.on("data", async (chunk: Buffer) => {
        process.stdout.write(chunk.toString());
        if (chunk.includes(LAUNCH_COMPLETE)) {
            await tail("9988.log", (chunk: string): boolean => {
                process.stdout.write(chunk);
                const match = chunk.match(PINT_FINALIZE);
                return (
                    match && match.length == 2 && Number.parseInt(match[1]) > 0
                );
            });

            console.log("FINALIZE SUCCEED!");
            process.exit(0);
        }
    });

    // Kill all processes when exiting.
    process.on("exit", () => {
        console.log("-> exit polkadot-launch...");
        killAll(ps, process.exitCode);
    });

    // Log errors
    ps.stderr.on("data", (chunk: Buffer) =>
        process.stderr.write(chunk.toString())
    );
}

(() => {
    main();
})();
