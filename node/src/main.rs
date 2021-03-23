// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

#![warn(unused_extern_crates)]

mod chain_spec;
#[macro_use]
mod service;
mod cli;
mod command;

fn main() -> sc_cli::Result<()> {
    command::run()
}
