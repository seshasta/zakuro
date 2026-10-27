//! a folder turned into a game of its own. an app.toml next to the program
//! names the game, which starts straight away with no library, and the
//! folder keeps everything the game needs and makes: its saves, the
//! settings, the system files dumped from a console and the recompiled
//! code. the folder can be moved or copied as one, and the program renamed
//! after the game. a folder without the files yet opens on setup, which
//! asks for them.
//!
//! ```toml
//! # the game, relative to this folder or a full path
//! game = "game/Tomodachi Life.3ds"
//! # the window's name, the game's own when left out
//! name = "Tomodachi Life"
//! # the game's title id, which setup takes no other game for
//! title_id = "000400000008C300"
//! ```

use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    game: PathBuf,
    name: Option<String>,
    title_id: Option<String>,
}

pub struct App {
    /// the folder the program and app.toml are in.
    pub dir: PathBuf,
    pub game: PathBuf,
    pub name: Option<String>,
    /// the game setup takes, none for any.
    pub title_id: Option<u64>,
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
    let title_id = match &manifest.title_id {
        Some(id) => Some(
            u64::from_str_radix(id.trim_start_matches("0x"), 16)
                .unwrap_or_else(|_| fail(&format!("app.toml's title_id, {id}, isn't a title id like 000400000008C300."))),
        ),
        None => None,
    };
    Some(App { game: dir.join(&manifest.game), name: manifest.name, title_id, dir })
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
