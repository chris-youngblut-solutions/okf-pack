//! Thin binary entry point. All logic lives in the `okf_pack` library crate so
//! it stays integration-testable.

fn main() -> anyhow::Result<()> {
    okf_pack::cli::run()
}
