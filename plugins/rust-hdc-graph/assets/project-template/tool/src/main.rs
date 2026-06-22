mod analyze;
mod hdc;
mod model;
mod rust_extract;
mod stable_hash;
mod wgsl_extract;

use std::env;
use std::path::PathBuf;

use anyhow::{Result, bail};

fn main() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();

    if args.is_empty() {
        print_usage();
        bail!("missing subcommand");
    }

    match args[0].as_str() {
        "extract" => {
            if args.len() < 2 || args.len() > 3 {
                print_usage();
                bail!("usage: rust-hdc-graph-tool extract <project-root> [out-dir]");
            }

            let root = PathBuf::from(&args[1]);
            let out_dir = args
                .get(2)
                .map(PathBuf::from)
                .unwrap_or_else(|| root.join("hdc-graph-out"));

            let artifact = analyze::analyze_project(&root)?;
            analyze::write_outputs(&artifact, &out_dir)?;

            println!("HDC graph complete.");
            println!("  root: {}", artifact.root);
            println!("  files: {}", artifact.summary.file_count);
            println!("  nodes: {}", artifact.summary.node_count);
            println!("  edges: {}", artifact.summary.edge_count);
            println!("  out: {}", out_dir.display());
        }
        "query" => {
            if args.len() != 3 {
                print_usage();
                bail!("usage: rust-hdc-graph-tool query <graph.json> <needle>");
            }
            let graph_path = PathBuf::from(&args[1]);
            let report = analyze::query_graph(&graph_path, &args[2])?;
            println!("{report}");
        }
        "context" => {
            if args.len() != 3 {
                print_usage();
                bail!("usage: rust-hdc-graph-tool context <graph.json> <task>");
            }
            let graph_path = PathBuf::from(&args[1]);
            let report = analyze::query_context(&graph_path, &args[2])?;
            println!("{report}");
        }
        _ => {
            print_usage();
            bail!("unknown subcommand: {}", args[0]);
        }
    }

    Ok(())
}

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  rust-hdc-graph-tool extract <project-root> [out-dir]");
    eprintln!("  rust-hdc-graph-tool query <graph.json> <needle>");
    eprintln!("  rust-hdc-graph-tool context <graph.json> <task>");
}
