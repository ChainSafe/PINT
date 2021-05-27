/**
 * pint-launch
 */
import { spawn, spawnSync, execSync } from "child_process";
import path from "path";

/**
 * Launch PINT locally or using docker in CI
 */
async function launch(docker: boolean = false) {
    if (!docker) {
        await local();
    }
}

/**
 * Launch PINT locally
 *
 * required bins:
 *  - npm
 *  - node
 */
async function local() {
    // build polkadot-launch
    execSync("npm run build", {
        cwd: path.resolve(__dirname, "../polkadot-launch"),
        stdio: "inherit",
    });
    spawnSync(
        "node",
        [
            path.resolve(__dirname, "../polkadot-launch/dist/index.js"),
            path.resolve(__dirname, "../../config.json"),
        ],
        {
            stdio: "inherit",
        }
    );
}
// async function docker() {}

launch();
