// tests for pallet asset-index
import { Result } from "../result";
import { Api } from "../api";
import { Pallet, Call } from "./index";

export const addAsset: Call = (api: Api): Result<null> => {
    return new Result(null);
};
