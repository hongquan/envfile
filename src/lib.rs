//! Libary for parsing environment files into an in-memory map.
//! 
//! ```rust
//! extern crate envfile;
//! 
//! use envfile::EnvFile;
//! use std::io;
//! use std::path::Path;
//! 
//! fn main() -> io::Result<()> {
//!     let mut envfile = EnvFile::new(&Path::new("examples/test.env"))?;
//! 
//!     for (key, value) in &envfile.store {
//!         println!("{}: {}", key, value);
//!     }
//! 
//!     envfile.update("ID", "example");
//!     println!("ID: {}", envfile.get("ID").unwrap_or(""));
//! 
//!     // envfile.write()?;
//! 
//!     Ok(())
//! }
//! ```

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use std::str;

/// An opened environment file, whose contents are buffered into memory.
pub struct EnvFile<'a> {
    /// Where the environment file exists in memory.
    pub path:  &'a Path,
    /// The data that was parsed from the file.
    pub store: BTreeMap<String, String>,
}

impl<'a> EnvFile<'a> {
    /// Open and parse an environment file.
    pub fn new(path: &'a Path) -> io::Result<EnvFile<'a>> {
        let data = read(path)?;
        let mut store = BTreeMap::new();

        let values = data.split(|&x| x == b'\n').flat_map(|entry| {
            entry.iter().position(|&x| x == b'=').and_then(|pos| {
                String::from_utf8(entry[..pos].to_owned()).ok()
                    .and_then(|x| {
                        String::from_utf8(entry[pos+1..].to_owned()).ok().map(|y| (x, y))
                    })
            })
        });

        for (key, value) in values {
            store.insert(key, value);
        }

        Ok(EnvFile { path, store })
    }

    /// Update or insert a key into the map.
    pub fn update(&mut self, key: &str, value: &str) {
        self.store.insert(key.into(), value.into());
    }

    /// Fetch a key from the map.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.store.get(key).as_ref().map(|x| x.as_str())
    }

    /// Write the map back to the original file.
    ///
    /// # Notes
    /// The keys are written in ascending order.
    pub fn write(&mut self) -> io::Result<()> {
        let mut buffer = Vec::with_capacity(1024);
        for (key, value) in &self.store {
            buffer.extend_from_slice(key.as_bytes());
            buffer.push(b'=');
            buffer.extend_from_slice(value.as_bytes());
            buffer.push(b'\n');
        }

        write(&self.path, &buffer)
    }
}

fn open<P: AsRef<Path>>(path: P) -> io::Result<File> {
    File::open(&path).map_err(|why| io::Error::new(
        io::ErrorKind::Other,
        format!("unable to open file at {:?}: {}", path.as_ref(), why)
    ))
}

fn create<P: AsRef<Path>>(path: P) -> io::Result<File> {
    File::create(&path).map_err(|why| io::Error::new(
        io::ErrorKind::Other,
        format!("unable to create file at {:?}: {}", path.as_ref(), why)
    ))
}

fn read<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    open(path).and_then(|mut file| {
        let mut buffer = Vec::with_capacity(file.metadata().ok().map_or(0, |x| x.len()) as usize);
        file.read_to_end(&mut buffer).map(|_| buffer)
    })
}

fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
    create(path).and_then(|mut file| file.write_all(contents.as_ref()))
}

#[cfg(test)]
mod tests {
    extern crate tempdir;
    use super::*;
    use self::tempdir::TempDir;
    use std::collections::BTreeMap;
    use std::io::Write;

    const SAMPLE: &str = r#"EFI_UUID=DFFD-D047
HOSTNAME=pop-testing
KBD_LAYOUT=us
KBD_MODEL=
KBD_VARIANT=
LANG=en_US.UTF-8
OEM_MODE=0
RECOVERY_UUID=PARTUUID=asdfasd7asdf7sad-asdfa
ROOT_UUID=2ef950c2-5ce6-4ae0-9fb9-a8c7468fa82c
"#;

    #[test]
    fn env_file_read() {
        let tempdir = TempDir::new("distinst_test").unwrap();
        let path = &tempdir.path().join("recovery.conf");

        {
            let mut file = create(path).unwrap();
            file.write_all(SAMPLE.as_bytes()).unwrap();
        }

        let env = EnvFile::new(path).unwrap();
        assert_eq!(&env.store, &{
            let mut map = BTreeMap::new();
            map.insert("HOSTNAME".into(), "pop-testing".into());
            map.insert("LANG".into(), "en_US.UTF-8".into());
            map.insert("KBD_LAYOUT".into(), "us".into());
            map.insert("KBD_MODEL".into(), "".into());
            map.insert("KBD_VARIANT".into(), "".into());
            map.insert("EFI_UUID".into(), "DFFD-D047".into());
            map.insert("RECOVERY_UUID".into(), "PARTUUID=asdfasd7asdf7sad-asdfa".into());
            map.insert("ROOT_UUID".into(), "2ef950c2-5ce6-4ae0-9fb9-a8c7468fa82c".into());
            map.insert("OEM_MODE".into(), "0".into());
            map
        });
    }

    #[test]
    fn env_file_write() {
        let tempdir = TempDir::new("distinst_test").unwrap();
        let path = &tempdir.path().join("recovery.conf");

        {
            let mut file = create(path).unwrap();
            file.write_all(SAMPLE.as_bytes()).unwrap();
        }

        let mut env = EnvFile::new(path).unwrap();
        env.write().unwrap();
        let copy: &[u8] = &read(path).unwrap();

        assert_eq!(copy, SAMPLE.as_bytes());
    }
}
