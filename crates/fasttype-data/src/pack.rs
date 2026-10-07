//! Format « pack » : plusieurs fichiers compressés en zstd dans un seul blob,
//! embarqué tel quel dans le binaire.
//!
//! Disposition : `FTPK\x01`, longueur de l'index (u32 petit-boutiste), index
//! JSON (`[PackEntry]`), puis les trames zstd concaténées. Les offsets sont
//! relatifs au début des trames.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::{self, Read, Write};

const MAGIC: &[u8; 5] = b"FTPK\x01";
/// Fenêtre de 128 Mio (`zstd --long=27`) : utile pour les grosses listes de mots.
const WINDOW_LOG: u32 = 27;
const LEVEL: i32 = 19;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackEntry {
    pub name: String,
    pub offset: u64,
    pub len: u64,
    pub raw_len: u64,
}

#[derive(Debug)]
pub enum PackError {
    BadMagic,
    Truncated,
    BadIndex(serde_json::Error),
    Io(io::Error),
    SizeMismatch { expected: u64, actual: u64 },
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackError::BadMagic => write!(f, "pack : en-tête invalide"),
            PackError::Truncated => write!(f, "pack : données tronquées"),
            PackError::BadIndex(e) => write!(f, "pack : index illisible ({e})"),
            PackError::Io(e) => write!(f, "pack : décompression impossible ({e})"),
            PackError::SizeMismatch { expected, actual } => {
                write!(
                    f,
                    "pack : taille décompressée {actual} au lieu de {expected}"
                )
            }
        }
    }
}

impl std::error::Error for PackError {}

impl From<io::Error> for PackError {
    fn from(e: io::Error) -> Self {
        PackError::Io(e)
    }
}

/// Compresse une trame (niveau 19, fenêtre longue, somme de contrôle pour
/// détecter une corruption). Sortie déterministe.
pub fn compress(raw: &[u8]) -> io::Result<Vec<u8>> {
    let mut enc = zstd::stream::Encoder::new(Vec::new(), LEVEL)?;
    enc.include_checksum(true)?;
    enc.window_log(WINDOW_LOG)?;
    enc.long_distance_matching(true)?;
    enc.write_all(raw)?;
    enc.finish()
}

/// Décompresse une trame et vérifie sa taille.
pub fn decompress_frame(frame: &[u8], raw_len: u64) -> Result<Vec<u8>, PackError> {
    let mut dec = zstd::stream::Decoder::new(frame)?;
    dec.window_log_max(WINDOW_LOG)?;
    let mut out = Vec::with_capacity(raw_len as usize);
    dec.read_to_end(&mut out)?;
    let actual = out.len() as u64;
    if actual != raw_len {
        return Err(PackError::SizeMismatch {
            expected: raw_len,
            actual,
        });
    }
    Ok(out)
}

#[derive(Default)]
pub struct PackWriter {
    entries: Vec<PackEntry>,
    payload: Vec<u8>,
}

impl PackWriter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ajoute un fichier ; les noms doivent être uniques. L'ordre d'ajout est
    /// conservé : l'appelant trie les noms pour un pack reproductible.
    pub fn add(&mut self, name: &str, raw: &[u8]) -> io::Result<()> {
        if self.entries.iter().any(|e| e.name == name) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("entrée en double : {name}"),
            ));
        }
        let frame = compress(raw)?;
        self.entries.push(PackEntry {
            name: name.to_string(),
            offset: self.payload.len() as u64,
            len: frame.len() as u64,
            raw_len: raw.len() as u64,
        });
        self.payload.extend_from_slice(&frame);
        Ok(())
    }

    pub fn finish(self) -> Vec<u8> {
        let index = serde_json::to_vec(&self.entries).expect("index sérialisable");
        let mut out = Vec::with_capacity(MAGIC.len() + 4 + index.len() + self.payload.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&(index.len() as u32).to_le_bytes());
        out.extend_from_slice(&index);
        out.extend_from_slice(&self.payload);
        out
    }
}

/// Vue en lecture seule d'un pack ; l'index est lu une fois, les trames à la demande.
pub struct Pack<'a> {
    entries: Vec<PackEntry>,
    payload: &'a [u8],
}

impl<'a> Pack<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, PackError> {
        let rest = bytes
            .strip_prefix(MAGIC.as_slice())
            .ok_or(PackError::BadMagic)?;
        let (len_bytes, rest) = rest.split_first_chunk::<4>().ok_or(PackError::Truncated)?;
        let index_len = u32::from_le_bytes(*len_bytes) as usize;
        if rest.len() < index_len {
            return Err(PackError::Truncated);
        }
        let (index, payload) = rest.split_at(index_len);
        let entries: Vec<PackEntry> = serde_json::from_slice(index).map_err(PackError::BadIndex)?;
        for e in &entries {
            let end = e.offset.checked_add(e.len).ok_or(PackError::Truncated)?;
            if end > payload.len() as u64 {
                return Err(PackError::Truncated);
            }
        }
        Ok(Self { entries, payload })
    }

    pub fn entries(&self) -> &[PackEntry] {
        &self.entries
    }

    pub fn get(&self, name: &str) -> Option<&PackEntry> {
        self.entries.iter().find(|e| e.name == name)
    }

    /// `Ok(None)` si l'entrée n'existe pas.
    pub fn decompress(&self, name: &str) -> Result<Option<Vec<u8>>, PackError> {
        let Some(e) = self.get(name) else {
            return Ok(None);
        };
        let frame = &self.payload[e.offset as usize..(e.offset + e.len) as usize];
        decompress_frame(frame, e.raw_len).map(Some)
    }
}
