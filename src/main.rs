//! Binary entry point for the `ch10r` CLI.

fn main() {
    irig106_ch10_reader::run_cli_with_binary_name(env!("CARGO_BIN_NAME"));
}
