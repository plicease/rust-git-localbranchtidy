use std::io::{self, Write};
use std::process::Command;

use anyhow::{Context, Result, bail};

const HELP: &str = "\
git-localbranchtidy - delete local branches already merged into the main branch

USAGE:
    git localbranchtidy [OPTIONS]

OPTIONS:
    -i, --interactive         Ask before removing each branch
        --main-branch <NAME>  Use <NAME> as the main branch instead of
                              auto-detecting it (usually `main` or `master`)
    -h, --help                Print this help and exit
    -V, --version             Print version and exit
";

struct Args {
    interactive: bool,
    main_branch: Option<String>,
}

fn parse_args() -> Result<Args> {
    let mut interactive = false;
    let mut main_branch = None;
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-i" | "--interactive" => interactive = true,
            "-h" | "--help" => {
                print!("{HELP}");
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--main-branch" => {
                let value = args
                    .next()
                    .context("--main-branch requires a branch name argument")?;
                main_branch = Some(value);
            }
            _ => {
                if let Some(value) = arg.strip_prefix("--main-branch=") {
                    main_branch = Some(value.to_string());
                } else {
                    bail!("unrecognized argument: {arg}\n\n{HELP}");
                }
            }
        }
    }

    Ok(Args {
        interactive,
        main_branch,
    })
}

/// Run a git command and return its captured stdout, erroring on a non-zero exit.
fn git(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .with_context(|| format!("failed to run `git {}`", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("`git {}` failed: {}", args.join(" "), stderr.trim());
    }

    String::from_utf8(output.stdout).context("git produced non-UTF-8 output")
}

fn local_branch_exists(name: &str) -> Result<bool> {
    let status = Command::new("git")
        .args(["show-ref", "--verify", "--quiet"])
        .arg(format!("refs/heads/{name}"))
        .status()
        .context("failed to run `git show-ref`")?;
    Ok(status.success())
}

/// Figure out which branch is the trunk.
///
/// Preference order:
///   1. `origin/HEAD` (what the remote considers its default branch)
///   2. a local branch named `main`
///   3. a local branch named `master`
fn detect_main_branch() -> Result<String> {
    if let Ok(out) = git(&["symbolic-ref", "--short", "refs/remotes/origin/HEAD"])
        && let Some(branch) = out.trim().strip_prefix("origin/")
        && local_branch_exists(branch)?
    {
        return Ok(branch.to_string());
    }

    for candidate in ["main", "master"] {
        if local_branch_exists(candidate)? {
            return Ok(candidate.to_string());
        }
    }

    bail!("could not detect the main branch; pass --main-branch <NAME> to specify it");
}

fn current_branch() -> Result<Option<String>> {
    let out = git(&["branch", "--show-current"])?;
    let name = out.trim();
    Ok((!name.is_empty()).then(|| name.to_string()))
}

/// Local branches whose tip is reachable from `main` (i.e. already merged),
/// excluding `main` itself and the currently checked-out branch.
fn merged_branches(main: &str) -> Result<Vec<String>> {
    let out = git(&["branch", "--merged", main, "--format=%(refname:short)"])?;
    let current = current_branch()?;

    let mut branches = Vec::new();
    for line in out.lines() {
        let name = line.trim();
        if name.is_empty() || name == "HEAD" || name == main {
            continue;
        }
        if Some(name) == current.as_deref() {
            continue;
        }
        branches.push(name.to_string());
    }
    Ok(branches)
}

fn confirm(prompt: &str) -> Result<bool> {
    print!("{prompt} [y/N] ");
    io::stdout().flush()?;

    let mut line = String::new();
    let read = io::stdin()
        .read_line(&mut line)
        .context("failed to read from stdin")?;
    if read == 0 {
        // EOF - treat as "no".
        println!();
        return Ok(false);
    }

    Ok(matches!(
        line.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn run() -> Result<()> {
    let args = parse_args()?;

    let main = match args.main_branch {
        Some(name) => {
            if !local_branch_exists(&name)? {
                bail!("main branch `{name}` does not exist locally");
            }
            name
        }
        None => detect_main_branch()?,
    };

    let branches = merged_branches(&main)?;
    if branches.is_empty() {
        println!("No merged branches to remove (main branch: {main}).");
        return Ok(());
    }

    for branch in branches {
        if args.interactive && !confirm(&format!("Delete branch '{branch}'?"))? {
            println!("Skipping '{branch}'.");
            continue;
        }

        let out = git(&["branch", "-d", &branch])?;
        let out = out.trim();
        if out.is_empty() {
            println!("Deleted branch '{branch}'.");
        } else {
            println!("{out}");
        }
    }

    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}
