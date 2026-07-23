use anyhow::{anyhow, Context, Result};
use scribefloat_lib::services::context_search::{
    build_index, default_model_cache_dir, export_context_pack, search_index, FastEmbedProvider,
    SearchOptions,
};
use scribefloat_lib::services::history::HistoryService;
use scribefloat_lib::types::Config;
use std::path::PathBuf;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        print_help();
        return Ok(());
    }

    let save_folder = take_option_value(&mut args, "--save-folder")
        .unwrap_or_else(|| Config::default().save_folder);

    match args.first().map(String::as_str) {
        Some("index") => run_index(&args[1..], &save_folder),
        Some("search") => run_search(&args[1..], &save_folder),
        Some("context") => run_context(&args[1..], &save_folder),
        Some(other) => Err(anyhow!("unknown command `{other}`")),
        None => Err(anyhow!("missing command")),
    }
}

fn run_index(args: &[String], save_folder: &str) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("build") => {
            let history = HistoryService::new();
            let mut provider = provider(save_folder)?;
            let manifest = build_index(&history, save_folder, &mut provider)?;
            println!(
                "indexed {} chunks with {} ({})",
                manifest.chunk_count, manifest.model_id, manifest.generated_at
            );
            Ok(())
        }
        Some(other) => Err(anyhow!("unknown index command `{other}`")),
        None => Err(anyhow!("missing index command; use `index build`")),
    }
}

fn run_search(args: &[String], save_folder: &str) -> Result<()> {
    let mut args = args.to_vec();
    let query = take_query(&mut args)?;
    let options = parse_search_options(query, &mut args)?;
    let mut provider = provider(save_folder)?;
    let results = search_index(save_folder, &mut provider, &options)?;
    for result in results {
        let timestamp = timestamp_label(result.start_ms, result.end_ms)
            .map(|ts| format!(" {ts}"))
            .unwrap_or_default();
        println!(
            "{:.3}\t{}{}\t{}\t{}",
            result.score.total, result.note_title, timestamp, result.note_id, result.snippet
        );
    }
    Ok(())
}

fn run_context(args: &[String], save_folder: &str) -> Result<()> {
    let mut args = args.to_vec();
    let query = take_option_value(&mut args, "--query")
        .or_else(|| take_option_value(&mut args, "-q"))
        .ok_or_else(|| anyhow!("context requires --query <text>"))?;
    let out = take_option_value(&mut args, "--out")
        .or_else(|| take_option_value(&mut args, "-o"))
        .ok_or_else(|| anyhow!("context requires --out <path>"))?;
    let options = parse_search_options(query, &mut args)?;
    let mut provider = provider(save_folder)?;
    let results = export_context_pack(save_folder, &mut provider, &options, &PathBuf::from(&out))?;
    println!("wrote {out} with {} chunks", results.len());
    Ok(())
}

fn provider(save_folder: &str) -> Result<FastEmbedProvider> {
    FastEmbedProvider::new_verified(default_model_cache_dir(save_folder))
}

fn parse_search_options(query: String, args: &mut Vec<String>) -> Result<SearchOptions> {
    let mut options = SearchOptions::new(query);
    if let Some(limit) = take_option_value(args, "--limit") {
        options.limit = limit
            .parse::<usize>()
            .with_context(|| format!("invalid --limit `{limit}`"))?;
    }
    if let Some(since) = take_option_value(args, "--since") {
        options.since_days = Some(parse_since_days(&since)?);
    }
    if let Some(tag) = take_option_value(args, "--tag") {
        options.tag = Some(tag);
    }
    if !args.is_empty() {
        return Err(anyhow!("unexpected arguments: {}", args.join(" ")));
    }
    Ok(options)
}

fn take_query(args: &mut Vec<String>) -> Result<String> {
    if let Some(query) =
        take_option_value(args, "--query").or_else(|| take_option_value(args, "-q"))
    {
        return Ok(query);
    }
    if args.is_empty() || args[0].starts_with('-') {
        return Err(anyhow!("search requires a query"));
    }
    Ok(args.remove(0))
}

fn take_option_value(args: &mut Vec<String>, flag: &str) -> Option<String> {
    let idx = args.iter().position(|arg| arg == flag)?;
    args.remove(idx);
    if idx >= args.len() {
        return None;
    }
    Some(args.remove(idx))
}

fn parse_since_days(value: &str) -> Result<i64> {
    let trimmed = value.trim();
    let number = trimmed
        .strip_suffix('d')
        .unwrap_or(trimmed)
        .parse::<i64>()
        .with_context(|| format!("invalid --since `{value}`; use a day count like 90d"))?;
    Ok(number)
}

fn timestamp_label(start_ms: Option<i64>, end_ms: Option<i64>) -> Option<String> {
    let start = start_ms?;
    let end = end_ms.unwrap_or(start);
    Some(format!("[{}-{}]", format_ms(start), format_ms(end)))
}

fn format_ms(ms: i64) -> String {
    let total_secs = (ms / 1000).max(0);
    format!("{:02}:{:02}", total_secs / 60, total_secs % 60)
}

fn print_help() {
    println!(
        "scribefloat-cli local context search\n\n\
Usage:\n  \
scribefloat-cli [--save-folder <path>] index build\n  \
scribefloat-cli [--save-folder <path>] search <query> [--limit 20] [--since 90d] [--tag tag]\n  \
scribefloat-cli [--save-folder <path>] context --query <query> --out <path> [--limit 12] [--since 90d] [--tag tag]\n"
    );
}
