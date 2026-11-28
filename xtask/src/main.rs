//! Build tasks, run with `cargo xtask <task>`.
//!
//! `cargo xtask build-web [game...] [--site DIR]` builds the browser version
//! of each game (size-optimised `web-release` profile for
//! `wasm32-unknown-unknown`, then `wasm-bindgen`, then `wasm-opt` when it is
//! installed) and stages a static site:
//!
//! ```text
//! site/index.html          landing page, one box per game
//! site/gameoflife/         Game of Life
//! site/tetris/             Tetris
//! site/sandsimulator/      Sand Simulator
//! site/pong/               Pong
//! ```

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const TARGET: &str = "wasm32-unknown-unknown";
const PROFILE: &str = "web-release";
/// Name of the wasm-bindgen output in each game folder (`game.js`, `game_bg.wasm`).
const OUTPUT_NAME: &str = "game";
/// The WebAssembly features Rust enables by default for `wasm32-unknown-unknown`.
const WASM_FEATURES: [&str; 6] = [
    "bulk-memory",
    "nontrapping-float-to-int",
    "sign-ext",
    "mutable-globals",
    "reference-types",
    "multivalue",
];

#[derive(Debug, PartialEq, Eq)]
struct Game {
    /// Folder in the site (and its URL path).
    path: &'static str,
    /// Workspace crate under `games/`.
    crate_name: &'static str,
    title: &'static str,
}

const GAMES: [Game; 4] = [
    Game {
        path: "gameoflife",
        crate_name: "game-of-life",
        title: "Game of Life",
    },
    Game {
        path: "tetris",
        crate_name: "tetris",
        title: "Tetris",
    },
    Game {
        path: "sandsimulator",
        crate_name: "sand-simulator",
        title: "Sand Simulator",
    },
    Game {
        path: "pong",
        crate_name: "pong",
        title: "Pong",
    },
];

/// The workspace root (this crate lives in `xtask/`).
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is in the workspace")
        .to_path_buf()
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn game_page(template: &str, game: &Game) -> String {
    template
        .replace("{{TITLE}}", &escape(game.title))
        .replace("{{NAME}}", OUTPUT_NAME)
}

fn landing_page(template: &str, games: &[&Game]) -> String {
    let items: Vec<String> = games
        .iter()
        .map(|game| {
            format!(
                "<li><a href=\"{}/\"><strong>{}</strong></a></li>",
                game.path,
                escape(game.title)
            )
        })
        .collect();
    template.replace("{{ITEMS}}", &items.join("\n"))
}

fn build_command(game: &Game) -> Vec<String> {
    [
        "cargo",
        "build",
        "--profile",
        PROFILE,
        "--target",
        TARGET,
        "-p",
        game.crate_name,
    ]
    .map(String::from)
    .to_vec()
}

fn wasm_path(root: &Path, game: &Game) -> PathBuf {
    root.join("target")
        .join(TARGET)
        .join(PROFILE)
        .join(format!("{}.wasm", game.crate_name))
}

fn bindgen_command(root: &Path, game: &Game, out_dir: &Path) -> Vec<String> {
    let mut command: Vec<String> = [
        "wasm-bindgen",
        "--target",
        "web",
        "--no-typescript",
        "--out-dir",
    ]
    .map(String::from)
    .to_vec();
    command.push(out_dir.display().to_string());
    command.extend(["--out-name".to_owned(), OUTPUT_NAME.to_owned()]);
    command.push(wasm_path(root, game).display().to_string());
    command
}

fn optimise_command(wasm: &Path) -> Vec<String> {
    let mut command = vec!["wasm-opt".to_owned(), "-Oz".to_owned()];
    command.extend(
        WASM_FEATURES
            .iter()
            .map(|feature| format!("--enable-{feature}")),
    );
    command.extend(["--strip-debug", "--strip-producers", "-o"].map(String::from));
    command.extend([wasm.display().to_string(), wasm.display().to_string()]);
    command
}

fn run(command: &[String], cwd: &Path) -> Result<(), String> {
    println!("$ {}", command.join(" "));
    let status = Command::new(&command[0])
        .args(&command[1..])
        .current_dir(cwd)
        .status()
        .map_err(|error| format!("{}: {error}", command[0]))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("{} failed: {status}", command[0]))
}

fn build_game(root: &Path, game: &Game, site: &Path, template: &str) -> Result<(), String> {
    let out_dir = site.join(game.path);
    if out_dir.exists() {
        fs::remove_dir_all(&out_dir).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&out_dir).map_err(|error| error.to_string())?;
    run(&build_command(game), root)?;
    run(&bindgen_command(root, game, &out_dir), root)?;
    // Optional size pass: an old or missing wasm-opt only costs download size.
    let wasm = out_dir.join(format!("{OUTPUT_NAME}_bg.wasm"));
    if let Err(error) = run(&optimise_command(&wasm), root) {
        println!("skipping the wasm-opt size pass ({error})");
    }
    fs::write(out_dir.join("index.html"), game_page(template, game)).map_err(|e| e.to_string())?;
    let size = fs::metadata(&wasm)
        .map_err(|error| error.to_string())?
        .len();
    println!("{}: {:.1} MB wasm", game.path, size as f64 / 1e6);
    Ok(())
}

/// The site folder and the games to build, from `build-web`'s arguments.
///
/// A relative `--site` is taken from `cwd`, where the command was run, and
/// made absolute: the build tools run from the workspace root.
fn parse_args(
    args: &[String],
    root: &Path,
    cwd: &Path,
) -> Result<(PathBuf, Vec<&'static Game>), String> {
    let mut site = root.join("site");
    let mut names = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if arg == "--site" {
            site = cwd.join(args.next().ok_or("--site needs a folder")?);
        } else {
            names.push(arg.as_str());
        }
    }
    let games = if names.is_empty() {
        GAMES.iter().collect()
    } else {
        names
            .iter()
            .map(|name| {
                GAMES.iter().find(|game| game.path == *name).ok_or_else(|| {
                    let known: Vec<&str> = GAMES.iter().map(|game| game.path).collect();
                    format!("unknown game {name}; choose from {}", known.join(", "))
                })
            })
            .collect::<Result<_, _>>()?
    };
    Ok((site, games))
}

fn build_web(args: &[String]) -> Result<(), String> {
    let root = root();
    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    let (site, games) = parse_args(args, &root, &cwd)?;
    let read =
        |name: &str| fs::read_to_string(root.join("web").join(name)).map_err(|e| e.to_string());
    let (game_template, landing_template) = (read("game.html")?, read("landing.html")?);
    for game in &games {
        build_game(&root, game, &site, &game_template)?;
    }
    let landed: Vec<&Game> = GAMES
        .iter()
        .filter(|game| site.join(game.path).is_dir())
        .collect();
    fs::write(
        site.join("index.html"),
        landing_page(&landing_template, &landed),
    )
    .map_err(|error| error.to_string())?;
    println!("site ready in {}", site.display());
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("build-web") => build_web(&args[1..]),
        _ => Err("usage: cargo xtask build-web [game...] [--site DIR]".to_owned()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests;
