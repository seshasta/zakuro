//! AES keys dumped from the user's own console. nothing here ships with
//! Zakuro: the keys are read from sysdata/aes_keys.txt, in the format Citra
//! and Azahar use, one `slot0x31KeyN=<32 hex digits>` (or a KeyX and KeyY
//! pair) per line. without the file the features that need a key stay
//! unavailable.

use std::collections::BTreeMap;
use std::path::Path;

/// where the key file is looked for, relative to the working directory and
/// then to the user's data directory, as the shared font is.
const KEY_FILE_PATHS: &[&str] = &["sysdata/aes_keys.txt", "aes_keys.txt"];

/// the constant the hardware key scrambler adds, from 3dbrew's AES registers
/// page.
const SCRAMBLER_C: u128 = 0x1FF9_E9AA_C5FE_0408_0245_91DC_5D52_768A;

/// the keyslot APT's Wrap and Unwrap encrypt with, Mii QR codes among others.
pub const APT_WRAP: u8 = 0x31;

#[derive(Default, Clone)]
pub struct AesKeys {
    normal: BTreeMap<u8, [u8; 16]>,
}

impl AesKeys {
    /// reads the first key file found, or no keys at all.
    pub fn load(data_dir: Option<&Path>) -> AesKeys {
        let text = KEY_FILE_PATHS
            .iter()
            .map(std::path::PathBuf::from)
            .chain(
                KEY_FILE_PATHS
                    .iter()
                    .filter_map(|path| Some(data_dir?.join(path))),
            )
            .find_map(|path| std::fs::read_to_string(path).ok());
        match text {
            Some(text) => {
                let keys = AesKeys::parse(&text);
                log::info!("aes keys: {} keyslots from aes_keys.txt", keys.normal.len());
                keys
            }
            None => AesKeys::default(),
        }
    }

    /// the normal keys a key file gives, each slot's own or the one its KeyX
    /// and KeyY make. unknown names and malformed lines are skipped.
    pub fn parse(text: &str) -> AesKeys {
        let mut normal = BTreeMap::new();
        let mut halves: BTreeMap<u8, (Option<u128>, Option<u128>)> = BTreeMap::new();
        for line in text.lines() {
            let Some((name, value)) = line.trim().split_once('=') else {
                continue;
            };
            let Some(rest) = name.trim().strip_prefix("slot0x") else {
                continue;
            };
            let Some(slot) = rest.get(..2).and_then(|s| u8::from_str_radix(s, 16).ok()) else {
                continue;
            };
            let value = value.trim();
            let Ok(key) = u128::from_str_radix(value, 16) else {
                continue;
            };
            if value.len() != 32 {
                continue;
            }
            match &rest[2..] {
                "KeyN" => {
                    normal.insert(slot, key.to_be_bytes());
                }
                "KeyX" => halves.entry(slot).or_default().0 = Some(key),
                "KeyY" => halves.entry(slot).or_default().1 = Some(key),
                _ => {}
            }
        }
        for (slot, halves) in halves {
            if let (Some(x), Some(y)) = halves {
                normal.entry(slot).or_insert_with(|| scramble(x, y));
            }
        }
        AesKeys { normal }
    }

    pub fn normal(&self, slot: u8) -> Option<[u8; 16]> {
        self.normal.get(&slot).copied()
    }
}

/// the normal key the hardware makes from a KeyX and a KeyY:
/// ((KeyX rol 2) xor KeyY) + C, rotated right by 41.
pub fn scramble(x: u128, y: u128) -> [u8; 16] {
    (x.rotate_left(2) ^ y)
        .wrapping_add(SCRAMBLER_C)
        .rotate_right(41)
        .to_be_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_normal_key_wins_over_its_halves() {
        let keys = AesKeys::parse(
            "slot0x31KeyX=00000000000000000000000000000001\n\
             slot0x31KeyY=00000000000000000000000000000002\n\
             slot0x31KeyN=000102030405060708090A0B0C0D0E0F\n",
        );
        assert_eq!(
            keys.normal(0x31),
            Some(*b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F")
        );
    }

    #[test]
    fn halves_make_the_scrambled_key() {
        let keys = AesKeys::parse("slot0x31KeyX=0\nslot0x31KeyX=00000000000000000000000000000000\nslot0x31KeyY=00000000000000000000000000000000\n");
        // with both halves zero the key is C rotated right by 41
        assert_eq!(
            keys.normal(0x31),
            Some(SCRAMBLER_C.rotate_right(41).to_be_bytes())
        );
    }

    #[test]
    fn junk_is_skipped() {
        let keys = AesKeys::parse(
            "# comment\nslot0x31KeyN=nothex\ncommon0=000102030405060708090A0B0C0D0E0F\n",
        );
        assert_eq!(keys.normal(0x31), None);
    }
}
