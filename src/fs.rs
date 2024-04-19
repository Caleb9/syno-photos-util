//! Isolates file-system operations for testing

use std::fs::{Metadata, Permissions};
use std::path::{Path, PathBuf};
use std::{fs, io};

pub trait Fs {
    fn home_dir(&self) -> Option<PathBuf>;
    fn exists(&self, path: &Path) -> bool;
    fn read_to_string<P: AsRef<Path>>(&self, path: P) -> io::Result<String>;
    fn write<P: AsRef<Path>, C: AsRef<[u8]>>(&self, path: P, contents: C) -> io::Result<()>;
    fn copy<P: AsRef<Path>, Q: AsRef<Path>>(&self, from: P, to: Q) -> io::Result<u64>;
    fn remove_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()>;
    fn set_permissions<P: AsRef<Path>>(&self, path: P, perm: Permissions) -> io::Result<()>;
    fn metadata<P: AsRef<Path>>(&self, path: P) -> io::Result<Metadata>;
}

pub struct FsImpl;

impl Fs for FsImpl {
    fn home_dir(&self) -> Option<PathBuf> {
        dirs::home_dir()
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn read_to_string<P: AsRef<Path>>(&self, path: P) -> io::Result<String> {
        fs::read_to_string(path)
    }

    fn write<P: AsRef<Path>, C: AsRef<[u8]>>(&self, path: P, contents: C) -> io::Result<()> {
        fs::write(path, contents)
    }

    fn copy<P: AsRef<Path>, Q: AsRef<Path>>(&self, from: P, to: Q) -> io::Result<u64> {
        fs::copy(from, to)
    }

    fn remove_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn set_permissions<P: AsRef<Path>>(&self, path: P, perm: Permissions) -> io::Result<()> {
        fs::set_permissions(path, perm)
    }

    fn metadata<P: AsRef<Path>>(&self, path: P) -> io::Result<Metadata> {
        fs::metadata(path)
    }
}
