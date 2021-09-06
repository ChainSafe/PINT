/**
 * Exports `pint-types-bundle` to `/resources/types.json`
 */

import fs from "fs";
import path from "path";
import { definitions } from "../pint-types-bundle/index";

(async () => {
    const types = (definitions.types as any)[0].types;
    if (types.length === 0) {
        throw "No PINT types provided";
    }

    const target = path.resolve(__dirname, "../../resources/types.json");
    fs.writeFileSync(target, JSON.stringify(types, null, 2));
    console.log("Updated \x1b[36m/resources/types.json\x1b[0m !");
})();
