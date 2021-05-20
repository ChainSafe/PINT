import yargs from "yargs/yargs";
import { hideBin } from "yargs/helpers";

/**
 * command entry
 */
const main = () => {
  const _ = yargs(hideBin(process.argv))
    .command({
      command: "run",
      describe: "run E2E tests",
      handler: () => {
        console.log("Run e2e tests");
      },
    })
    .help()
    .demandCommand(1, "You need at least one command before moving on").argv;
};

// exports
export default main;
