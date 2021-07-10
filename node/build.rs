// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only
use std::fs;

const PINT_TYPES_SOURCE: &'static str = "../js/pint-types-bundle/pint.json";
const PINT_TYPES_TARGET: &'static str = "../resources/types.json";

use substrate_build_script_utils::{generate_cargo_keys, rerun_if_git_head_changed};

fn main() {
    generate_cargo_keys();
    rerun_if_git_head_changed();

    // Generate PINT types
    fs::copy(PINT_TYPES_SOURCE, PINT_TYPES_TARGET).expect("Generate PINT types failed");
}
