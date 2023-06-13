use super::{FileOrDirectory, OpenFileType};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Default, Clone, Deserialize, Serialize)]
pub struct ChildPaths {
    pub children: Vec<FileOrDirectory>,
    pub selected: Option<usize>,
}

impl ChildPaths {
    /// Set the child paths and selection.
    pub fn set(
        &mut self,
        directory: &Path,
        open_file_type: &OpenFileType,
        previous_directory: Option<PathBuf>,
    ) {
        // Get the paths.
        let children = self.get_paths_in_directory(directory, open_file_type);
        // Get the folders. This sets the selection.
        let folders: Vec<&FileOrDirectory> = children.iter().filter(|p| !p.is_file).collect();
        // Set the selection index.
        self.selected = match previous_directory {
            // Select the previous directory.
            Some(previous_directory) => children
                .iter()
                .enumerate()
                .filter(|p| p.1.path == previous_directory)
                .map(|p| p.0)
                .next(),
            // Select the first file if possible. Otherwise select the first folder if possible.
            None => match !children.is_empty() {
                true => {
                    // Start at the first file.
                    match (folders.is_empty(), !children.iter().any(|p| p.is_file)) {
                        (true, true) => None,
                        (true, false) => Some(0),
                        (false, true) => Some(0),
                        (false, false) => Some(folders.len()),
                    }
                }
                false => None,
            },
        };
        self.children = children;
    }

    /// Get the child paths of a directory.
    fn get_paths_in_directory(
        &self,
        directory: &Path,
        open_file_type: &OpenFileType,
    ) -> Vec<FileOrDirectory> {
        // Get the file extensions.
        let extensions = open_file_type.get_extensions();
        // Find all valid paths.
        let valid_paths: Vec<PathBuf> = match directory.read_dir() {
            Ok(read) => read
                .filter(|e| e.is_ok())
                .map(|e| e.unwrap().path())
                .filter(|p| p.is_file() || p.read_dir().is_ok())
                .collect(),
            Err(_) => vec![],
        };
        // Get the files.
        let mut files: Vec<&PathBuf> = valid_paths
            .iter()
            .filter(|p| {
                p.is_file()
                    && p.extension().is_some()
                    && extensions.contains(&p.extension().unwrap().to_str().unwrap())
            })
            .collect();
        files.sort();
        // Get the directories.
        let mut folders: Vec<&PathBuf> = valid_paths.iter().filter(|p| p.is_dir()).collect();
        folders.sort();

        let mut paths: Vec<FileOrDirectory> =
            folders.iter().map(|f| FileOrDirectory::new(f)).collect();
        paths.append(&mut files.iter().map(|f| FileOrDirectory::new(f)).collect());
        paths
    }
}
