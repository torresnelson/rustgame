use super::*;

fn template(name: &str) -> String {
    fs::read_to_string(root().join("web").join(name)).unwrap()
}

#[test]
fn every_game_is_a_workspace_crate() {
    for game in &GAMES {
        let manifest = fs::read_to_string(
            root()
                .join("games")
                .join(game.crate_name)
                .join("Cargo.toml"),
        )
        .unwrap();
        assert!(
            manifest.contains(&format!("name = \"{}\"", game.crate_name)),
            "{}",
            game.crate_name
        );
    }
}

#[test]
fn the_size_profile_exists() {
    let manifest = fs::read_to_string(root().join("Cargo.toml")).unwrap();
    assert!(manifest.contains(&format!("[profile.{PROFILE}]")));
}

#[test]
fn game_page_fills_title_and_module_name() {
    let html = game_page(&template("game.html"), &GAMES[0]);
    assert!(html.contains("<title>Game of Life</title>"));
    assert!(html.contains(&format!("import init from './{OUTPUT_NAME}.js';")));
    assert!(html.contains(&format!("'./{OUTPUT_NAME}_bg.wasm'")));
    assert!(!html.contains("{{"));
}

#[test]
fn game_page_has_the_canvas_loading_screen_and_phone_settings() {
    let html = template("game.html");
    assert!(html.contains("<canvas id=\"bevy\"")); // sandbox_ui::CANVAS_SELECTOR
    assert!(html.contains("window.hide_loading = ")); // called by sandbox_ui::web::hide_loading
    assert_eq!(html.matches("<meta name=\"viewport\"").count(), 1);
    assert!(html.contains("width=device-width"));
    assert!(html.contains("touch-action: none"));
    // `?debug` shows the log on screen, for phones without developer tools.
    assert!(html.contains(".has('debug')"));
    // The wasm reaches `init` as a stream, so the browser compiles while it
    // downloads; wasm-bindgen streams only `application/wasm` responses.
    assert!(html.contains("new Response(counted"));
    // sandbox_ui::audio plays into the context made on the first gesture.
    assert!(html.contains("window.gameAudio = new Context()"));
    assert!(html.contains("'Content-Type': 'application/wasm'"));
}

#[test]
fn landing_page_is_just_the_game_boxes() {
    let games: Vec<&Game> = GAMES.iter().collect();
    let html = landing_page(&template("landing.html"), &games);
    assert_eq!(html.matches("<li>").count(), GAMES.len());
    for game in &GAMES {
        assert!(html.contains(&format!(
            "<li><a href=\"{}/\"><strong>{}</strong></a></li>",
            game.path, game.title
        )));
    }
    // Boxes as wide as the longest title, text centred (as on the pygame site).
    assert!(html.contains("width: max-content") && html.contains("text-align: center"));
    assert!(!html.contains("{{"));
}

#[test]
fn landing_page_escapes_titles() {
    let odd = Game {
        path: "x",
        crate_name: "x",
        title: "A <b> & C",
    };
    let html = landing_page("{{ITEMS}}", &[&odd]);
    assert!(html.contains("A &lt;b&gt; &amp; C"));
}

#[test]
fn commands_build_bind_and_optimise() {
    let game = GAMES
        .iter()
        .find(|game| game.path == "sandsimulator")
        .unwrap();
    let build = build_command(game);
    assert_eq!(&build[..4], ["cargo", "build", "--profile", PROFILE]);
    assert_eq!(build.last().unwrap(), game.crate_name);
    let bindgen = bindgen_command(Path::new("/w/target"), game, Path::new("/site/x"));
    assert_eq!(bindgen[0], "wasm-bindgen");
    assert_eq!(
        bindgen.last().unwrap(),
        "/w/target/wasm32-unknown-unknown/web-release/sand-simulator.wasm"
    );
    let optimise = optimise_command(Path::new("/site/x/game_bg.wasm"));
    assert_eq!(&optimise[..2], ["wasm-opt", "-Oz"]);
    assert!(optimise.contains(&"--enable-reference-types".to_owned()));
}

#[test]
fn locked_version_reads_the_lockfile() {
    let lock = "[[package]]\nname = \"wasm-bindgen\"\nversion = \"0.2.130\"\n\
                [[package]]\nname = \"wasm-bindgen-shared\"\nversion = \"0.2.130\"\n";
    assert_eq!(
        locked_version(lock, "wasm-bindgen").as_deref(),
        Some("0.2.130")
    );
    assert_eq!(locked_version(lock, "serde"), None);
    // The real lockfile has it (Pages installs the CLI from the same line).
    let real = fs::read_to_string(root().join("Cargo.lock")).unwrap();
    assert!(locked_version(&real, "wasm-bindgen").is_some());
}

#[test]
fn check_bindgen_names_the_install_command() {
    assert_eq!(
        check_bindgen(Some("wasm-bindgen 0.2.130\n"), "0.2.130"),
        Ok(())
    );
    let old = check_bindgen(Some("wasm-bindgen 0.2.100\n"), "0.2.130").unwrap_err();
    assert!(old.contains("0.2.100") && old.contains("--version 0.2.130 --locked"));
    let missing = check_bindgen(None, "0.2.130").unwrap_err();
    assert!(missing.contains("not installed"));
}

#[test]
fn target_dir_follows_cargo_target_dir() {
    let root = Path::new("/w");
    assert_eq!(target_dir(root, None), Path::new("/w/target"));
    assert_eq!(
        target_dir(root, Some("/cache/target".into())),
        Path::new("/cache/target")
    );
    // Relative to the workspace root, where cargo runs.
    assert_eq!(target_dir(root, Some("out".into())), Path::new("/w/out"));
}

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|&arg| arg.to_owned()).collect()
}

#[test]
fn parse_args_defaults_to_every_game_in_the_root_site() {
    let (site, games) = parse_args(&[], Path::new("/w"), Path::new("/w/xtask")).unwrap();
    assert_eq!(site, Path::new("/w/site"));
    assert_eq!(games.len(), GAMES.len());
}

#[test]
fn parse_args_picks_games_and_resolves_the_site_from_the_caller() {
    let (site, games) = parse_args(
        &args(&["tetris", "--site", "out"]),
        Path::new("/w"),
        Path::new("/home/me"),
    )
    .unwrap();
    assert_eq!(site, Path::new("/home/me/out"));
    assert_eq!(games, [&GAMES[1]]);
    let (site, _) =
        parse_args(&args(&["--site", "/abs"]), Path::new("/w"), Path::new("/x")).unwrap();
    assert_eq!(site, Path::new("/abs"));
}

#[test]
fn parse_args_rejects_unknown_games_and_a_missing_folder() {
    let root = Path::new("/w");
    let unknown = parse_args(&args(&["asteroids"]), root, root).unwrap_err();
    assert!(unknown.contains("unknown game asteroids"), "{unknown}");
    assert!(parse_args(&args(&["--site"]), root, root).is_err());
}

/// Emoji and pictographic symbols (arrows, shapes, dingbats, emoji ranges,
/// variation selectors, zero-width joiner); same rule as the pygame repo.
fn is_emoji(char: char) -> bool {
    const RANGES: [(u32, u32); 8] = [
        (0x2190, 0x21FF),
        (0x2300, 0x23FF),
        (0x25A0, 0x25FF),
        (0x2600, 0x27BF),
        (0x2B00, 0x2BFF),
        (0x1F000, 0x1FAFF),
        (0xFE0E, 0xFE0F),
        (0x200D, 0x200D),
    ];
    RANGES
        .iter()
        .any(|&(low, high)| (low..=high).contains(&(char as u32)))
}

fn text_files(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            text_files(&path, found);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| ["rs", "toml", "md", "html", "yml", "nix"].contains(&e))
        {
            found.push(path);
        }
    }
}

#[test]
fn the_repository_has_no_emoji() {
    let root = root();
    let mut files = vec![
        root.join("Cargo.toml"),
        root.join("README.md"),
        root.join("shell.nix"),
    ];
    for folder in ["crates", "games", "xtask", "web", "plans", ".github"] {
        text_files(&root.join(folder), &mut files);
    }
    let found: Vec<String> = files
        .iter()
        .filter_map(|path| {
            let text = fs::read_to_string(path).ok()?;
            text.lines()
                .position(|line| line.chars().any(is_emoji))
                .map(|line| format!("{}:{}", path.display(), line + 1))
        })
        .collect();
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn the_emoji_rule_catches_emoji_but_not_plain_text() {
    for code in [0x23CF, 0x25B6, 0x1F600, 0xFE0F] {
        assert!(is_emoji(char::from_u32(code).unwrap()), "{code:x}");
    }
    assert!(
        !"plain ascii -> <- [ ] and accents: \u{e9}"
            .chars()
            .any(is_emoji)
    );
}
