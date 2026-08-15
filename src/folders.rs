//! Folder tree derived from vault paths.

use crate::storage::Note;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct FolderTree {
    pub root: FolderNode,
}

#[derive(Debug, Default)]
pub struct FolderNode {
    pub name: String,
    pub relative_path: PathBuf,
    pub folders: Vec<FolderNode>,
    pub note_ids: Vec<Uuid>,
}

#[derive(Default)]
struct FolderBuilder {
    folders: BTreeMap<OsString, FolderBuilder>,
    note_ids: Vec<Uuid>,
}

impl FolderTree {
    pub fn build(notes: &[Note], notes_root: &Path, known_folders: &[PathBuf]) -> Self {
        let mut builder = FolderBuilder::default();

        // Physical paths preserve empty folders.
        for folder in known_folders {
            if let Some(components) = normal_components(folder) {
                builder.folder_mut(&components);
            }
        }

        for note in notes {
            let Some(parent) = note.file_path.parent() else {
                continue;
            };
            let Ok(relative_parent) = parent.strip_prefix(notes_root) else {
                continue;
            };
            let Some(components) = normal_components(relative_parent) else {
                continue;
            };
            builder.folder_mut(&components).note_ids.push(note.id);
        }

        Self {
            root: builder.into_node("Notes".to_owned(), PathBuf::new()),
        }
    }
}

impl FolderBuilder {
    fn folder_mut(&mut self, components: &[OsString]) -> &mut Self {
        let mut current = self;
        for component in components {
            current = current.folders.entry(component.clone()).or_default();
        }
        current
    }

    fn into_node(self, name: String, relative_path: PathBuf) -> FolderNode {
        let folders = self
            .folders
            .into_iter()
            .map(|(child_name, child)| {
                let child_path = relative_path.join(&child_name);
                child.into_node(child_name.to_string_lossy().into_owned(), child_path)
            })
            .collect();

        FolderNode {
            name,
            relative_path,
            folders,
            note_ids: self.note_ids,
        }
    }
}

fn normal_components(path: &Path) -> Option<Vec<OsString>> {
    path.components()
        .map(|component| match component {
            Component::Normal(name) => Some(name.to_os_string()),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_nested_tree_and_keeps_empty_folders() {
        let root = PathBuf::from("vault/Notes");
        let mut rust = Note::new_named(&root.join("Programming"), "Rust Tips");
        rust.file_path = root.join("Programming/Rust Tips--1234abcd.md");
        let mut bones = Note::new_named(&root.join("Biologia/Anatomy"), "Bones");
        bones.file_path = root.join("Biologia/Anatomy/Bones--1234abcd.md");

        let tree = FolderTree::build(
            &[rust, bones],
            &root,
            &[
                PathBuf::new(),
                PathBuf::from("Programming"),
                PathBuf::from("Biologia"),
                PathBuf::from("Biologia/Anatomy"),
                PathBuf::from("Empty"),
            ],
        );

        assert_eq!(tree.root.folders.len(), 3);
        let biologia = tree
            .root
            .folders
            .iter()
            .find(|folder| folder.name == "Biologia")
            .expect("Biologia folder");
        assert_eq!(biologia.folders[0].name, "Anatomy");
        assert_eq!(biologia.folders[0].note_ids.len(), 1);
        assert!(
            tree.root
                .folders
                .iter()
                .any(|folder| folder.name == "Empty" && folder.note_ids.is_empty())
        );
    }
}
