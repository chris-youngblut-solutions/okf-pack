//! Command-line surface (`clap`).
//!
//! KC-1a wires the full subcommand shape with stub handlers; each later queue
//! item fills one in. The stubs return a clear "not yet implemented" error
//! (naming the responsible queue item) rather than printing — keeping the CLI
//! shape real and `--help` honest before the internals land.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// okf-pack — knowledge-context format + adapter + Spaces pack.
#[derive(Debug, Parser)]
#[command(
    name = "okf-pack",
    version,
    about = "OKF-compatible knowledge-context format + adapter + Spaces pack"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// The okf-pack subcommand surface (see the design plan, Part B).
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Validate a bundle against the okf-ext schema (`--fix` rewrites canonical).
    Validate {
        bundle: PathBuf,
        #[arg(long)]
        fix: bool,
    },
    /// Lint a bundle for closed-vocabulary and typed-link discipline.
    Lint { bundle: PathBuf },
    /// Import an OKF bundle into the internal (okf-ext) form.
    Import {
        okf_bundle: PathBuf,
        dest: PathBuf,
        /// Force a specific surface adapter for a foreign bundle.
        #[arg(long)]
        surface: Option<String>,
    },
    /// Export an internal bundle to canonical OKF.
    Export {
        bundle: PathBuf,
        dest: PathBuf,
        #[arg(long, value_enum)]
        target: ExportTarget,
        /// Emit without the `.ckf/` sidecar (pure hand-off; lossy).
        #[arg(long)]
        no_sidecar: bool,
    },
    /// Read a live operator surface into an internal bundle.
    MigrateSurface {
        surface: Surface,
        path: PathBuf,
        /// Destination bundle dir (default: emit beside, never in place).
        #[arg(long)]
        to: Option<PathBuf>,
        /// Rewrite the source surface in place (requires a clean git tree).
        #[arg(long)]
        in_place: bool,
    },
    /// Build and emit the link graph.
    Graph {
        bundle: PathBuf,
        #[arg(long, value_enum, default_value_t = GraphEmit::Json)]
        emit: GraphEmit,
    },
    /// Round-trip fidelity check (the CI / loop gate).
    Roundtrip { bundle: PathBuf },
    /// Scaffold a new knowledge-context pack into a target directory.
    Init {
        target: PathBuf,
        #[arg(long, default_value = "spaces.knowledge-context")]
        pack_id: String,
        #[arg(long, default_value = "knowledge-context")]
        component: String,
        #[arg(long, default_value = "local")]
        embedder: String,
    },
    /// Run the Spaces pack RPC server.
    Serve {
        #[arg(long, value_enum, default_value_t = RpcTransport::Stdio)]
        rpc: RpcTransport,
    },
}

/// Where an OKF export is allowed to go (gates the privilege denylist).
#[derive(Debug, Clone, ValueEnum)]
pub enum ExportTarget {
    /// Local files only — always permitted, even for privileged corpora.
    FilesOnly,
    /// Google Cloud Knowledge Catalog ingestion — blocked for privileged content.
    OkfGcp,
}

/// A live operator knowledge surface the adapter can read.
#[derive(Debug, Clone, ValueEnum)]
pub enum Surface {
    Memory,
    Skills,
    Devtel,
    Container,
    Okf,
}

/// Graph output backends. (The `kuzu` backend was dropped — see ADR 0001.)
#[derive(Debug, Clone, ValueEnum)]
pub enum GraphEmit {
    Json,
    Html,
}

/// RPC transports the pack server can speak.
#[derive(Debug, Clone, ValueEnum)]
pub enum RpcTransport {
    Stdio,
}

/// Parse argv and dispatch. Entry point for the `okf-pack` binary.
///
/// # Errors
/// Returns an error for any subcommand whose implementation has not landed yet
/// (and, once implemented, for that subcommand's own failure modes).
pub fn run() -> anyhow::Result<()> {
    dispatch(Cli::parse().command)
}

fn dispatch(command: Command) -> anyhow::Result<()> {
    match command {
        Command::Validate { bundle, fix: _ } => {
            let errors = crate::surface::validate_bundle(&bundle)?;
            if errors.is_empty() {
                println!("ok: {} is a valid okf-ext bundle", bundle.display());
                Ok(())
            } else {
                for e in &errors {
                    eprintln!("{e}");
                }
                anyhow::bail!("{} validation error(s)", errors.len())
            }
        }
        Command::MigrateSurface {
            surface,
            path,
            to,
            in_place,
        } => {
            if in_place {
                anyhow::bail!(
                    "--in-place is not yet implemented (emit-to-bundle only; pass --to <dir>)"
                );
            }
            let dest = to.ok_or_else(|| anyhow::anyhow!("--to <dir> is required"))?;
            let name = surface_name(&surface);
            let count = crate::surface::migrate(name, &path, &dest)?;
            println!(
                "migrated {count} note(s) from `{name}` surface -> {}",
                dest.display()
            );
            Ok(())
        }
        Command::Lint { .. } => not_yet("lint", "KC-9"),
        Command::Import {
            okf_bundle,
            dest,
            surface: _,
        } => {
            let count = crate::okf::import(&okf_bundle, &dest)?;
            println!(
                "imported {count} note(s) from {} -> {}",
                okf_bundle.display(),
                dest.display()
            );
            Ok(())
        }
        Command::Export {
            bundle,
            dest,
            target,
            no_sidecar,
        } => {
            let denylist = crate::privilege::Denylist::load()?;
            let target = match target {
                ExportTarget::FilesOnly => crate::okf::Target::FilesOnly,
                ExportTarget::OkfGcp => crate::okf::Target::OkfGcp,
            };
            let report = crate::okf::export(&bundle, &dest, target, &denylist, !no_sidecar)?;
            println!(
                "exported {} note(s) -> {} (sha256 {}…, sidecar {})",
                report.item_count,
                dest.display(),
                &report.sha256[..16],
                report.sidecar
            );
            Ok(())
        }
        Command::Graph { bundle, emit } => {
            let graph = crate::graph::build(&bundle)?;
            let out = match emit {
                GraphEmit::Json => crate::graph::to_json(&graph)?,
                GraphEmit::Html => crate::graph::to_html(&graph)?,
            };
            println!("{out}");
            Ok(())
        }
        Command::Roundtrip { bundle } => {
            let errors = crate::canonical::roundtrip_bundle(&bundle)?;
            if errors.is_empty() {
                println!("ok: {} round-trips losslessly", bundle.display());
                Ok(())
            } else {
                for e in &errors {
                    eprintln!("{e}");
                }
                anyhow::bail!("{} round-trip failure(s)", errors.len())
            }
        }
        Command::Init {
            target,
            pack_id,
            component,
            embedder,
        } => {
            let opts =
                crate::scaffold::InitOptions::new(target.clone(), pack_id, component, embedder);
            let count = crate::scaffold::init(&opts)?;
            println!("scaffolded {count} file(s) into {}", target.display());
            Ok(())
        }
        Command::Serve { rpc: _ } => crate::rpc::serve_stdio(),
    }
}

fn surface_name(surface: &Surface) -> &'static str {
    match surface {
        Surface::Memory => "memory",
        Surface::Skills => "skills",
        Surface::Devtel => "devtel",
        Surface::Container => "container",
        Surface::Okf => "okf",
    }
}

fn not_yet(command: &str, item: &str) -> anyhow::Result<()> {
    anyhow::bail!("`{command}` is not yet implemented (queue item {item})")
}
