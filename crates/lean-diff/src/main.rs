use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::ExitCode;

use lean_corpus::{ReadError, RecordReader};

const USAGE: &str = "\
usage: lean-diff --function <fqn> --corpus <path>

  --function <fqn>    Lean function whose corpus to replay (e.g. Lean.Meta.isLevelDefEqAuxImpl)
  --corpus  <path>    path to the corpus file (typically <fqn>.bin)
  --limit   <N>       stop after N records (default: all)
  --help              show this help and exit
";

struct Args {
    function: String,
    corpus: PathBuf,
    limit: Option<usize>,
}

fn parse_args() -> Result<Args, String> {
    let mut function = None;
    let mut corpus = None;
    let mut limit = None;
    let mut argv = std::env::args().skip(1);
    while let Some(arg) = argv.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                print!("{USAGE}");
                std::process::exit(0);
            }
            "--function" => {
                function = Some(argv.next().ok_or("--function requires a value")?);
            }
            "--corpus" => {
                corpus = Some(PathBuf::from(
                    argv.next().ok_or("--corpus requires a value")?,
                ));
            }
            "--limit" => {
                let v = argv.next().ok_or("--limit requires a value")?;
                limit = Some(v.parse::<usize>().map_err(|e| format!("--limit: {e}"))?);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(Args {
        function: function.ok_or("--function is required")?,
        corpus: corpus.ok_or("--corpus is required")?,
        limit,
    })
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    let file = match File::open(&args.corpus) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: opening {}: {e}", args.corpus.display());
            return ExitCode::from(1);
        }
    };
    let reader = match RecordReader::open(BufReader::new(file)) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: reading {}: {e}", args.corpus.display());
            return ExitCode::from(1);
        }
    };

    let pin_sha_hex: String = reader
        .header
        .pin_sha
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    eprintln!(
        "lean-diff: function={} pinned={} format=v{}",
        args.function, pin_sha_hex, reader.header.version
    );

    let mut count: usize = 0;
    for record in reader {
        match record {
            Ok(_r) => {
                count += 1;
                if args.limit.is_some_and(|n| count >= n) {
                    break;
                }
            }
            Err(ReadError::Io(e)) => {
                eprintln!("error: io while reading record {}: {e}", count + 1);
                return ExitCode::from(1);
            }
            Err(ReadError::Format(e)) => {
                eprintln!("error: format at record {}: {e}", count + 1);
                return ExitCode::from(1);
            }
        }
    }

    eprintln!("lean-diff: {count} record(s) walked, dispatch not yet implemented");
    ExitCode::SUCCESS
}
