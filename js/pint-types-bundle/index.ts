import {
    OverrideBundleDefinition,
    OverrideBundleType,
} from "@polkadot/types/types";
import PINTTypes from "./pint.json";

export const definitions = {
    types: [
        {
            // on all versions
            minmax: [0, undefined],
            types: PINTTypes,
        },
    ],
} as OverrideBundleDefinition;

export const typesBundle = {
    spec: {
        pint: definitions,
    },
} as OverrideBundleType;
