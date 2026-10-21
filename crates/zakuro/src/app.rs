//! a folder turned into a game of its own. an app.toml next to the program
//! names the game, which starts straight away with no library, and the
//! folder keeps everything the game needs and makes: its saves, the
//! settings, the system files dumped from a console and the recompiled
//! code. the folder can be moved or copied as one, and the program renamed
//! after the game.
//!
//! ```toml
//! # the game, relative to this folder or a full path
//! game = "game/Tomodachi Life.3ds"
//! # the window's name, the game's own when left out
//! name = "Tomodachi Life"
//! ```

use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    game: PathBuf,
    name: Option<String>,
}

pub struct App {
    /// the folder the program and app.toml are in.
    pub dir: PathBuf,
    pub game: PathBuf,
    pub name: Option<String>,
}

impl App {
    /// the folder recompiled code is kept in, <title id>.dll in it.
    pub fn recompiled(&self) -> PathBuf {
        self.dir.join("recompiled")
    }
}

static APP: OnceLock<Option<App>> = OnceLock::new();

/// the app this program runs as, when there is an app.toml next to it.
pub fn get() -> Option<&'static App> {
    APP.get_or_init(load).as_ref()
}

fn load() -> Option<App> {
    let dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    let text = std::fs::read_to_string(dir.join("app.toml")).ok()?;
    let manifest: Manifest = toml::from_str(&text).unwrap_or_else(|error| fail(&format!("app.toml could not be read.\n\n{error}")));
    Some(App { game: dir.join(&manifest.game), name: manifest.name, dir })
}

/// makes sure the game is where app.toml says, before any window opens.
pub fn check(app: &App) {
    if !app.game.is_file() {
        fail(&format!(
            "The game isn't where app.toml says it is:\n{}\n\nPut your own decrypted dump of the game there, or change `game` in app.toml.",
            app.game.display()
        ));
    }
}

/// tells the player what stops the game from starting, in a window since a
/// program started from a shortcut shows no console, and ends the program.
pub fn fail(message: &str) -> ! {
    log::error!("{message}");
    let title = get_name().unwrap_or("Zakuro");
    rfd::MessageDialog::new()
        .set_title(title)
        .set_description(message)
        .set_level(rfd::MessageLevel::Error)
        .show();
    std::process::exit(1);
}

/// the app's own name, while app.toml is being read too.
fn get_name() -> Option<&'static str> {
    APP.get()?.as_ref()?.name.as_deref()
}
