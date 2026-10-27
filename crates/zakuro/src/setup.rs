//! the first run of a folder made into a game of its own, see app: the
//! files the game needs that only the player's own copy of it and their own
//! console can give, which the folder gets by the player choosing them or
//! dropping them on the window. each is checked for being what it should be
//! before it is copied in, a system font converted from the file the
//! console keeps it in first, and the game's code is recompiled from the
//! same screen.

use std::path::{Path, PathBuf};

use crate::app::App;

/// a file the folder needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Part {
    Game,
    Font,
    Miis,
    Keys,
}

impl Part {
    pub const ALL: [Part; 4] = [Part::Game, Part::Font, Part::Miis, Part::Keys];

    pub fn name(self) -> &'static str {
        match self {
            Part::Game => "The game",
            Part::Font => "System font",
            Part::Miis => "Mii resources",
            Part::Keys => "AES keys",
        }
    }

    /// whether the game can start without it.
    pub fn required(self) -> bool {
        self != Part::Keys
    }

    /// what it is for.
    pub fn purpose(self) -> &'static str {
        match self {
            Part::Game => "Your own decrypted dump of the game, a .3ds, .cci, .cxi or .cia.",
            Part::Font => "The 3DS's own font, which all the game's text is drawn in.",
            Part::Miis => "The models and textures Miis' heads are drawn from.",
            Part::Keys => "Optional: only needed to scan Mii QR codes.",
        }
    }

    /// how to get it from a console, with GodMode9.
    pub fn how(self) -> &'static str {
        match self {
            Part::Game => {
                "In GodMode9, find the game (a cartridge under [C:] GAMECART, or an installed one under [A:] SYSNAND SD > title > 00040000), \
                 press A on it and choose \"Build CIA from this file\" or \"Dump gamecart\", then on the dump choose \"Decrypt file\". \
                 The decrypted file is in gm9/out on the SD card."
            }
            Part::Font => {
                "In GodMode9, go to [1:] SYSNAND CTRNAND > title > 0004009b > 00014002 > content, press A on the .app file and choose \
                 \"Mount as NCCH image\", open romfs, then press A on cbf_std.bcfnt.lz and copy it to 0:/gm9/out. \
                 Choose that file here, it's converted for you."
            }
            Part::Miis => {
                "In GodMode9, go to [1:] SYSNAND CTRNAND > title > 0004009b > 00010202 > content, press A on the .app file and choose \
                 \"Mount as NCCH image\", open romfs, then press A on CFL_Res.dat and copy it to 0:/gm9/out."
            }
            Part::Keys => {
                "A key file in the format Citra and Azahar use, with the key for slot 0x31 (slot0x31KeyN, or slot0x31KeyX and \
                 slot0x31KeyY), worked out from your own console's boot9.bin and firmware."
            }
        }
    }

    /// where the folder keeps it.
    pub fn file(self, app: &App) -> PathBuf {
        match self {
            Part::Game => app.game.clone(),
            Part::Font => app.dir.join("sysdata/shared_font.bin"),
            Part::Miis => app.dir.join("sysdata/CFL_Res.dat"),
            Part::Keys => app.dir.join("sysdata/aes_keys.txt"),
        }
    }
}

/// how a part stands in the folder.
#[derive(Debug, Clone, PartialEq)]
pub enum State {
    /// there and right, and a word on what it is.
    Ready(String),
    Missing,
    /// there and wrong, and why.
    Wrong(String),
}

impl State {
    pub fn ready(&self) -> bool {
        matches!(self, State::Ready(_))
    }
}

/// how a part the folder has stands.
pub fn check(app: &App, part: Part) -> State {
    let file = part.file(app);
    if !file.is_file() {
        return State::Missing;
    }
    let result = match part {
        Part::Game => game(app, &file),
        _ => match std::fs::read(&file) {
            Ok(data) => examine(part, &data).map(|(_, about)| about),
            Err(error) => Err(format!("it can't be read: {error}")),
        },
    };
    match result {
        Ok(about) => State::Ready(about),
        Err(why) => State::Wrong(why),
    }
}

/// the game's title id, from the game in the folder.
pub fn program_id(app: &App) -> Option<u64> {
    zakuro_fs::Title::load(&app.game).ok().map(|title| title.program_id())
}

/// whether the folder has the game's code recompiled.
pub fn recompiled(app: &App, program_id: u64) -> bool {
    app.recompiled().join(library_name(program_id)).is_file()
}

/// what setup has found and is doing.
pub struct Setup {
    /// each part and how it stands.
    pub states: Vec<(Part, State)>,
    /// the game's title id, once the folder has it.
    pub program_id: Option<u64>,
    /// whether the folder has the game's code recompiled.
    pub recompiled: bool,
    /// whether this computer has it recompiled from before, outside the
    /// folder, ready to copy in.
    pub recompiled_before: bool,
    /// the part being copied in, on a thread of its own, the game can take
    /// a while, and those chosen or dropped meanwhile.
    copying: Option<(Part, std::thread::JoinHandle<Result<String, String>>)>,
    waiting: std::collections::VecDeque<(Part, PathBuf)>,
    /// how the last file chosen or dropped went, what it turned out to be,
    /// and whether it went in.
    pub last: Option<(Option<Part>, Result<String, String>)>,
    /// recompiling needs a compiler downloaded, which the player is asked
    /// about.
    pub needs_compiler: bool,
}

impl Setup {
    pub fn new(app: &App) -> Setup {
        let mut setup = Setup {
            states: Vec::new(),
            program_id: None,
            recompiled: false,
            recompiled_before: false,
            copying: None,
            waiting: Default::default(),
            last: None,
            needs_compiler: false,
        };
        setup.refresh(app);
        setup
    }

    /// looks at the folder again.
    pub fn refresh(&mut self, app: &App) {
        self.states = Part::ALL.iter().map(|&part| (part, check(app, part))).collect();
        let game = self.states.iter().any(|(part, state)| *part == Part::Game && state.ready());
        self.program_id = game.then(|| program_id(app)).flatten();
        self.recompiled = self.program_id.is_some_and(|id| recompiled(app, id));
        self.recompiled_before = !self.recompiled && self.program_id.and_then(zakuro_core::recompiled::installed).is_some();
    }

    /// whether the game can start.
    pub fn ready(&self) -> bool {
        self.copying.is_none() && self.waiting.is_empty() && self.states.iter().all(|(part, state)| !part.required() || state.ready())
    }

    /// the part being copied in, if one is.
    pub fn copying(&self) -> Option<Part> {
        self.copying.as_ref().map(|(part, _)| *part)
    }

    /// copies a file the player chose for a part into the folder, once the
    /// one before is in.
    pub fn choose(&mut self, app: &'static App, part: Part, path: PathBuf) {
        self.waiting.push_back((part, path));
        self.next(app);
    }

    /// copies a file dropped on the window into the folder as whatever part
    /// it is.
    pub fn dropped(&mut self, app: &'static App, path: PathBuf) {
        match identify(&path) {
            Some(part) => self.choose(app, part, path),
            None => {
                let name = path.file_name().map_or_else(|| path.display().to_string(), |name| name.to_string_lossy().into_owned());
                self.last = Some((None, Err(format!("{name} isn't any of the files setup asks for."))));
            }
        }
    }

    /// whether a copy finished since, the folder looked at again then.
    pub fn poll(&mut self, app: &'static App) -> bool {
        let Some((_, thread)) = &self.copying else { return false };
        if !thread.is_finished() {
            return false;
        }
        let Some((part, thread)) = self.copying.take() else { return false };
        let result = thread.join().unwrap_or_else(|_| Err("copying it went wrong".to_owned()));
        self.last = Some((Some(part), result));
        self.refresh(app);
        self.next(app);
        true
    }

    fn next(&mut self, app: &'static App) {
        if self.copying.is_some() {
            return;
        }
        if let Some((part, path)) = self.waiting.pop_front() {
            self.copying = Some((part, std::thread::spawn(move || install(app, part, &path))));
        }
    }
}

/// the file a game's recompiled code goes by.
pub fn library_name(program_id: u64) -> String {
    format!("{program_id:016X}.{}", std::env::consts::DLL_EXTENSION)
}

/// which part a file is, going by what is in it, for a file dropped on the
/// window.
pub fn identify(path: &Path) -> Option<Part> {
    let size = std::fs::metadata(path).ok()?.len();
    // what the small parts are is read out of them whole, the game is told
    // by its headers
    if size <= 16 << 20 {
        let data = std::fs::read(path).ok()?;
        for part in [Part::Keys, Part::Font, Part::Miis] {
            if examine(part, &data).is_ok() || looks_like(part, path, &data) {
                return Some(part);
            }
        }
    }
    zakuro_fs::Title::load(path).is_ok().then_some(Part::Game)
}

/// whether a file is plainly meant to be a part, though it may be the wrong
/// one, so that it is checked as that part and the player told why it isn't
/// right.
fn looks_like(part: Part, path: &Path, data: &[u8]) -> bool {
    let name = path.file_name().map(|name| name.to_string_lossy().to_lowercase()).unwrap_or_default();
    match part {
        Part::Keys => name.ends_with(".txt") && String::from_utf8_lossy(data).contains("slot0x"),
        Part::Font => name.contains("bcfnt") || name.contains("shared_font") || font_start(data).is_some(),
        Part::Miis => name.contains("cfl_res"),
        Part::Game => false,
    }
}

/// copies a part into the folder from a file, once it is what it should
/// be, a font converted on the way, and says what it is. a file there before
/// is kept beside it with .old added.
pub fn install(app: &App, part: Part, from: &Path) -> Result<String, String> {
    let to = part.file(app);
    if let Some(dir) = to.parent() {
        std::fs::create_dir_all(dir).map_err(|error| format!("{} can't be made: {error}", dir.display()))?;
    }
    let keep_old = |to: &Path| -> Result<(), String> {
        if to.is_file() {
            let mut old = to.as_os_str().to_owned();
            old.push(".old");
            std::fs::rename(to, &old).map_err(|error| format!("the file there can't be moved aside: {error}"))?;
        }
        Ok(())
    };
    match part {
        Part::Game => {
            let about = game(app, from)?;
            if std::fs::canonicalize(from).ok() != std::fs::canonicalize(&to).ok() {
                keep_old(&to)?;
                std::fs::copy(from, &to).map_err(|error| format!("it can't be copied: {error}"))?;
            }
            Ok(about)
        }
        _ => {
            let data = std::fs::read(from).map_err(|error| format!("it can't be read: {error}"))?;
            let (data, about) = examine(part, &data)?;
            keep_old(&to)?;
            std::fs::write(&to, data).map_err(|error| format!("it can't be written: {error}"))?;
            Ok(about)
        }
    }
}

/// what the game at path is, when it is the folder's game and can be run.
fn game(app: &App, path: &Path) -> Result<String, String> {
    let title = zakuro_fs::Title::load(path).map_err(|error| match error {
        zakuro_fs::FsError::Encrypted(_) | zakuro_fs::FsError::EncryptedCia | zakuro_fs::FsError::UnreadableRomFs(_) => {
            "this dump is encrypted. In GodMode9, press A on it and choose \"Decrypt file\", then use the decrypted one.".to_owned()
        }
        zakuro_fs::FsError::UnknownFormat => "this isn't a 3DS game dump.".to_owned(),
        error => format!("it can't be read as a game: {error}."),
    })?;
    let id = title.program_id();
    let name = title.exefs_file("icon").and_then(crate::library::Smdh::parse).map(|smdh| smdh.name).filter(|name| !name.is_empty());
    let name = name.unwrap_or_else(|| format!("{id:016X}"));
    if let Some(wanted) = app.title_id {
        if id != wanted {
            return Err(format!("this is {name} ({id:016X}), and this folder is for {wanted:016X}."));
        }
    }
    Ok(format!("{name}, {id:016X}"))
}

/// the bytes a part is kept as, from a file meant to be it, and what it is,
/// or why it isn't that part.
fn examine(part: Part, data: &[u8]) -> Result<(Vec<u8>, String), String> {
    match part {
        Part::Font => font(data),
        Part::Miis => {
            // the console's is about a megabyte, homebrew makes a small
            // stand-in with no heads in it
            if data.len() < 512 * 1024 {
                return Err(format!(
                    "this file is only {} KB, a stand-in some homebrew makes, not your console's, which is about a megabyte.",
                    data.len() / 1024
                ));
            }
            if data.get(..4) != Some(&[0x14, 0x00, 0x09, 0x05][..]) {
                return Err("this isn't a CFL_Res.dat.".to_owned());
            }
            Ok((data.to_vec(), format!("{} KB", data.len() / 1024)))
        }
        Part::Keys => {
            let text = std::str::from_utf8(data).map_err(|_| "this isn't a key file.".to_owned())?;
            if !text.contains("slot0x") {
                return Err("this isn't a key file.".to_owned());
            }
            if zakuro_core::keys::AesKeys::parse(text).normal(zakuro_core::keys::APT_WRAP).is_none() {
                return Err("this key file doesn't have the key for slot 0x31, which Mii QR codes are read with.".to_owned());
            }
            Ok((data.to_vec(), "Mii QR codes can be read".to_owned()))
        }
        Part::Game => Err("a game is checked from its file".to_owned()),
    }
}

/// where a font starts in a file, a bare one or one after the 0x80 bytes
/// the system puts before it in memory.
fn font_start(data: &[u8]) -> Option<usize> {
    [0, 0x80].into_iter().find(|&at| matches!(data.get(at..at + 4), Some(b"CFNT" | b"CFNU")))
}

/// the shared font as Zakuro maps it, the 0x80 bytes the system puts before
/// it, which say it is loaded, and the font, its magic CFNU as in memory,
/// from a dump of it, cbf_std.bcfnt or the compressed one the console keeps
/// it in. a font without the accented letters the game's text has, such as
/// one made for a plugin, is turned down.
fn font(data: &[u8]) -> Result<(Vec<u8>, String), String> {
    let unpacked;
    let data = match font_start(data) {
        Some(_) => data,
        None if data.first() == Some(&0x11) => {
            unpacked = lz11(data).ok_or("this file is damaged, it doesn't unpack.")?;
            &unpacked[..]
        }
        None => return Err("this isn't a 3DS font.".to_owned()),
    };
    let start = font_start(data).ok_or("this isn't a 3DS font.")?;
    let mut font = data[start..].to_vec();
    let characters = characters(&font).ok_or("this font is damaged.")?;
    let has = |c: char| characters.contains(&(c as u16));
    if !"èàçüñ".chars().all(has) {
        return Err(
            "this font is missing accented letters (è, à, ç...), so it isn't a 3DS's own system font; one made for a plugin can be like that."
                .to_owned(),
        );
    }
    font[..4].copy_from_slice(b"CFNU");
    let mut out = vec![0u8; 0x80];
    out[0..4].copy_from_slice(&2u32.to_le_bytes());
    out[4..8].copy_from_slice(&1u32.to_le_bytes());
    out[8..12].copy_from_slice(&(font.len() as u32).to_le_bytes());
    out.extend_from_slice(&font);
    Ok((out, format!("{} characters", characters.len())))
}

/// the characters a font has, from its character maps, whose pointers are
/// from the font's start or from where it was laid out in memory.
fn characters(font: &[u8]) -> Option<Vec<u16>> {
    let u16_at = |at: usize| Some(u16::from_le_bytes(font.get(at..at + 2)?.try_into().ok()?));
    let u32_at = |at: usize| Some(u32::from_le_bytes(font.get(at..at + 4)?.try_into().ok()?));
    let finf = u16_at(6)? as usize;
    if font.get(finf..finf + 4)? != b"FINF" {
        return None;
    }
    // FINF's pointer to TGLP, against where TGLP is, tells how far the
    // pointers are from the font's own offsets
    let sections = u32_at(16)?;
    let mut at = finf;
    let mut tglp = None;
    for _ in 0..sections.min(16) {
        if font.get(at..at + 4)? == b"TGLP" {
            tglp = Some(at);
            break;
        }
        at += u32_at(at + 4)? as usize;
    }
    let shift = u32_at(finf + 16)?.wrapping_sub(tglp? as u32 + 8);
    let mut codes = Vec::new();
    let mut map = u32_at(finf + 24)?;
    for _ in 0..256 {
        if map == 0 {
            break;
        }
        let body = map.wrapping_sub(shift) as usize;
        let (begin, end, method) = (u16_at(body)?, u16_at(body + 2)?, u16_at(body + 4)?);
        let table = body + 12;
        match method {
            0 => codes.extend(begin..=end),
            1 => {
                for (k, code) in (begin..=end).enumerate() {
                    if u16_at(table + 2 * k)? != 0xFFFF {
                        codes.push(code);
                    }
                }
            }
            _ => {
                for k in 0..u16_at(table)? as usize {
                    codes.push(u16_at(table + 2 + 4 * k)?);
                }
            }
        }
        map = u32_at(body + 8)?;
    }
    Some(codes)
}

/// unpacks LZ11, as the console keeps its font.
fn lz11(data: &[u8]) -> Option<Vec<u8>> {
    let mut size = u32::from_le_bytes([*data.get(1)?, *data.get(2)?, *data.get(3)?, 0]) as usize;
    let mut at = 4;
    if size == 0 {
        size = u32::from_le_bytes(data.get(4..8)?.try_into().ok()?) as usize;
        at = 8;
    }
    if size > 64 << 20 {
        return None;
    }
    let mut out = Vec::with_capacity(size);
    while out.len() < size {
        let flags = *data.get(at)?;
        at += 1;
        for bit in 0..8 {
            if out.len() >= size {
                break;
            }
            if flags & (0x80 >> bit) == 0 {
                out.push(*data.get(at)?);
                at += 1;
                continue;
            }
            let b0 = *data.get(at)? as usize;
            let (length, distance) = match b0 >> 4 {
                0 => {
                    let (b1, b2) = (*data.get(at + 1)? as usize, *data.get(at + 2)? as usize);
                    at += 3;
                    (((b0 & 0xF) << 4 | b1 >> 4) + 0x11, ((b1 & 0xF) << 8 | b2) + 1)
                }
                1 => {
                    let (b1, b2, b3) = (*data.get(at + 1)? as usize, *data.get(at + 2)? as usize, *data.get(at + 3)? as usize);
                    at += 4;
                    (((b0 & 0xF) << 12 | b1 << 4 | b2 >> 4) + 0x111, ((b2 & 0xF) << 8 | b3) + 1)
                }
                kind => {
                    let b1 = *data.get(at + 1)? as usize;
                    at += 2;
                    (kind + 1, ((b0 & 0xF) << 8 | b1) + 1)
                }
            };
            for _ in 0..length {
                let byte = *out.get(out.len().checked_sub(distance)?)?;
                out.push(byte);
            }
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// a small font with one character map of each kind, its pointers
    /// from its start, as a .bcfnt has them.
    fn tiny_font(scanned: &[u16]) -> Vec<u8> {
        let mut font = Vec::new();
        font.extend_from_slice(b"CFNT");
        font.extend_from_slice(&[0xFF, 0xFE, 0x14, 0x00, 0, 0, 0, 3]);
        font.extend_from_slice(&0u32.to_le_bytes()); // file size, patched below
        font.extend_from_slice(&3u32.to_le_bytes()); // sections
        // FINF at 0x14, 0x20 long, TGLP right after it at 0x34
        let finf = font.len();
        font.extend_from_slice(b"FINF");
        font.extend_from_slice(&0x20u32.to_le_bytes());
        font.extend_from_slice(&[1, 0x1E, 0, 0, 0, 0x18, 0x18, 1]);
        font.extend_from_slice(&(0x34u32 + 8).to_le_bytes()); // TGLP
        font.extend_from_slice(&0u32.to_le_bytes()); // CWDH
        font.extend_from_slice(&0u32.to_le_bytes()); // CMAP, patched below
        font.extend_from_slice(&[0; 4]);
        assert_eq!(font.len(), 0x34);
        font.extend_from_slice(b"TGLP");
        font.extend_from_slice(&0x10u32.to_le_bytes());
        font.extend_from_slice(&[0; 8]);
        // a direct map of ASCII, then one scanned map
        let direct = font.len() + 8;
        font.extend_from_slice(b"CMAP");
        font.extend_from_slice(&0x18u32.to_le_bytes());
        font.extend_from_slice(&0x20u16.to_le_bytes());
        font.extend_from_slice(&0x7Eu16.to_le_bytes());
        font.extend_from_slice(&0u16.to_le_bytes());
        font.extend_from_slice(&0u16.to_le_bytes());
        let next = font.len();
        font.extend_from_slice(&0u32.to_le_bytes());
        font.extend_from_slice(&0u32.to_le_bytes());
        let scan = font.len() + 8;
        font.extend_from_slice(b"CMAP");
        font.extend_from_slice(&((20 + 4 * scanned.len()) as u32).to_le_bytes());
        font.extend_from_slice(&0u16.to_le_bytes());
        font.extend_from_slice(&0xFFFFu16.to_le_bytes());
        font.extend_from_slice(&2u16.to_le_bytes());
        font.extend_from_slice(&0u16.to_le_bytes());
        font.extend_from_slice(&0u32.to_le_bytes());
        font.extend_from_slice(&(scanned.len() as u16).to_le_bytes());
        for (index, &code) in scanned.iter().enumerate() {
            font.extend_from_slice(&code.to_le_bytes());
            font.extend_from_slice(&(100 + index as u16).to_le_bytes());
        }
        font[next..next + 4].copy_from_slice(&(scan as u32).to_le_bytes());
        font[finf + 24..finf + 28].copy_from_slice(&(direct as u32).to_le_bytes());
        let size = font.len() as u32;
        font[12..16].copy_from_slice(&size.to_le_bytes());
        font
    }

    /// LZ11 of data with every byte stored as it is.
    fn stored(data: &[u8]) -> Vec<u8> {
        let mut out = vec![0x11, data.len() as u8, (data.len() >> 8) as u8, (data.len() >> 16) as u8];
        for chunk in data.chunks(8) {
            out.push(0);
            out.extend_from_slice(chunk);
        }
        out
    }

    #[test]
    fn a_console_font_is_converted_and_a_plugin_one_turned_down() {
        let accents: Vec<u16> = "èàçüñé".chars().map(|c| c as u16).collect();
        let console = tiny_font(&accents);
        let (shared, about) = font(&console).expect("the console's font");
        assert_eq!(&shared[..12], &[2, 0, 0, 0, 1, 0, 0, 0, console.len() as u8, (console.len() >> 8) as u8, 0, 0]);
        assert_eq!(&shared[0x80..0x84], b"CFNU");
        assert_eq!(&shared[0x84..], &console[4..]);
        assert_eq!(about, format!("{} characters", 0x7E - 0x20 + 1 + accents.len()));
        // the same font compressed, and as a shared_font.bin, read the same
        assert_eq!(font(&stored(&console)).unwrap().0, shared);
        assert_eq!(font(&shared).unwrap().0, shared);
        // a plugin's font with é alone
        let plugin = tiny_font(&['é' as u16]);
        assert!(font(&plugin).unwrap_err().contains("accented"));
    }

    #[test]
    fn a_stand_in_for_the_mii_resources_is_turned_down() {
        let mut real = vec![0u8; 1_083_400];
        real[..4].copy_from_slice(&[0x14, 0x00, 0x09, 0x05]);
        assert!(examine(Part::Miis, &real).is_ok());
        let stand_in = vec![0u8; 73 * 1024];
        assert!(examine(Part::Miis, &stand_in).unwrap_err().contains("stand-in"));
    }

    #[test]
    fn a_key_file_needs_the_key_qr_codes_are_read_with() {
        let with = format!("slot0x31KeyN={}\n", "00112233445566778899AABBCCDDEEFF");
        assert!(examine(Part::Keys, with.as_bytes()).is_ok());
        let without = "slot0x2CKeyX=00112233445566778899AABBCCDDEEFF\n";
        assert!(examine(Part::Keys, without.as_bytes()).unwrap_err().contains("0x31"));
    }
}

