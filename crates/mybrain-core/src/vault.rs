//! The [`Vault`]: an in-memory mirror of a folder of Markdown notes.
//!
//! All mutations go through the vault so the on-disk files and the in-memory
//! state never diverge. Writes are atomic (write to a temp file, then rename).

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::error::{Error, Result};
use crate::links::parse_links;
use crate::note::{Note, NoteId, NOTE_EXTENSION};

/// A search result returned by [`Vault::search`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub id: NoteId,
    /// `true` if the query matched the note title.
    pub title_match: bool,
    /// First matching line of the body, if the body matched.
    pub snippet: Option<String>,
}

/// A folder of Markdown notes.
#[derive(Debug)]
pub struct Vault {
    root: PathBuf,
    notes: BTreeMap<NoteId, Note>,
    resolver: LinkResolver,
}

impl Vault {
    /// Opens (and creates if necessary) the vault located at `root` and loads
    /// every Markdown file in it. Hidden folders such as `.git` or `.obsidian`
    /// are skipped.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|e| Error::io(&root, e))?;
        let mut vault = Self {
            root,
            notes: BTreeMap::new(),
            resolver: LinkResolver::default(),
        };
        vault.reload()?;
        Ok(vault)
    }

    /// Re-reads every note from disk.
    pub fn reload(&mut self) -> Result<()> {
        let mut notes = BTreeMap::new();
        let walker = WalkDir::new(&self.root)
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| e.depth() == 0 || !is_hidden(e.path()));

        for entry in walker.filter_map(std::result::Result::ok) {
            let path = entry.path();
            if !entry.file_type().is_file()
                || path.extension().and_then(|e| e.to_str()) != Some(NOTE_EXTENSION)
            {
                continue;
            }
            let Ok(relative) = path.strip_prefix(&self.root) else {
                continue;
            };
            let Ok(id) = NoteId::from_relative_path(relative) else {
                continue; // file name we cannot represent as a note: ignore it
            };
            let content = fs::read_to_string(path).map_err(|e| Error::io(path, e))?;
            let modified = entry.metadata().ok().and_then(|m| m.modified().ok());
            notes.insert(
                id.clone(),
                Note {
                    id,
                    content,
                    modified,
                },
            );
        }

        self.notes = notes;
        self.rebuild_resolver();
        Ok(())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Absolute path of the file backing `id`.
    pub fn path_of(&self, id: &NoteId) -> PathBuf {
        self.root.join(id.relative_path())
    }

    /// All notes, sorted by id.
    pub fn notes(&self) -> impl Iterator<Item = &Note> {
        self.notes.values()
    }

    pub fn len(&self) -> usize {
        self.notes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    pub fn get(&self, id: &NoteId) -> Option<&Note> {
        self.notes.get(id)
    }

    pub fn contains(&self, id: &NoteId) -> bool {
        self.notes.contains_key(id)
    }

    /// Creates a new note. Fails if it already exists.
    pub fn create(&mut self, id: NoteId, content: &str) -> Result<&Note> {
        if self.contains(&id) {
            return Err(Error::AlreadyExists(id.to_string()));
        }
        self.write(id.clone(), content)?;
        self.rebuild_resolver();
        Ok(&self.notes[&id])
    }

    /// Writes `content` to the note, creating it if needed.
    pub fn save(&mut self, id: &NoteId, content: &str) -> Result<()> {
        let is_new = !self.contains(id);
        self.write(id.clone(), content)?;
        if is_new {
            self.rebuild_resolver();
        }
        Ok(())
    }

    /// Appends `text` on a new line at the end of the note (creating it if needed).
    pub fn append(&mut self, id: &NoteId, text: &str) -> Result<()> {
        let mut content = self.get(id).map(|n| n.content.clone()).unwrap_or_default();
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(text);
        if !content.ends_with('\n') {
            content.push('\n');
        }
        self.save(id, &content)
    }

    /// Deletes a note from disk and memory.
    pub fn delete(&mut self, id: &NoteId) -> Result<()> {
        if self.notes.remove(id).is_none() {
            return Err(Error::NotFound(id.to_string()));
        }
        let path = self.path_of(id);
        fs::remove_file(&path).map_err(|e| Error::io(&path, e))?;
        self.rebuild_resolver();
        Ok(())
    }

    /// Renames a note and rewrites every `[[link]]` pointing to it.
    ///
    /// Returns the number of *other* notes whose links were updated.
    pub fn rename(&mut self, from: &NoteId, to: NoteId) -> Result<usize> {
        if from == &to {
            return Ok(0);
        }
        if self.contains(&to) {
            return Err(Error::AlreadyExists(to.to_string()));
        }
        let note = self
            .notes
            .remove(from)
            .ok_or_else(|| Error::NotFound(from.to_string()))?;

        // Find links to `from` *before* the resolver is updated.
        let referrers: Vec<(NoteId, String)> = self
            .notes
            .values()
            .filter_map(|n| {
                let updated = self.rewrite_links(&n.content, from, &to);
                (updated != n.content).then(|| (n.id.clone(), updated))
            })
            .collect();

        let old_path = self.path_of(from);
        let new_path = self.path_of(&to);
        ensure_parent(&new_path)?;
        fs::rename(&old_path, &new_path).map_err(|e| Error::io(&old_path, e))?;
        self.notes.insert(
            to.clone(),
            Note {
                id: to.clone(),
                ..note
            },
        );

        for (id, content) in &referrers {
            self.write(id.clone(), content)?;
        }
        self.rebuild_resolver();
        Ok(referrers.len())
    }

    /// Resolves a link target (as written inside `[[...]]`) to a note.
    ///
    /// Resolution is case-insensitive and follows Obsidian's rules: an exact
    /// path match wins, otherwise the note with that file name and the
    /// shortest path is chosen.
    pub fn resolve_link(&self, target: &str) -> Option<&NoteId> {
        self.resolver.resolve(target)
    }

    /// Returns a fresh id that does not collide with an existing note, by
    /// appending ` 2`, ` 3`, ... to `base` if needed.
    pub fn unique_id(&self, base: &NoteId) -> NoteId {
        if !self.contains(base) {
            return base.clone();
        }
        (2..)
            .filter_map(|n| NoteId::new(&format!("{base} {n}")).ok())
            .find(|id| !self.contains(id))
            .expect("an unused id always exists")
    }

    /// Case-insensitive full-text search over titles and bodies.
    pub fn search(&self, query: &str) -> Vec<SearchHit> {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return Vec::new();
        }
        let mut hits: Vec<SearchHit> = self
            .notes
            .values()
            .filter_map(|note| {
                let title_match = note.id.as_str().to_lowercase().contains(&query);
                let snippet = note
                    .body()
                    .lines()
                    .find(|l| l.to_lowercase().contains(&query))
                    .map(|l| l.trim().to_owned());
                (title_match || snippet.is_some()).then(|| SearchHit {
                    id: note.id.clone(),
                    title_match,
                    snippet,
                })
            })
            .collect();
        // Title matches first, then alphabetical.
        hits.sort_by(|a, b| b.title_match.cmp(&a.title_match).then(a.id.cmp(&b.id)));
        hits
    }

    // ----------------------------------------------------------------- private

    fn write(&mut self, id: NoteId, content: &str) -> Result<()> {
        let path = self.path_of(&id);
        ensure_parent(&path)?;
        atomic_write(&path, content)?;
        let modified = fs::metadata(&path).ok().and_then(|m| m.modified().ok());
        self.notes.insert(
            id.clone(),
            Note {
                id,
                content: content.to_owned(),
                modified,
            },
        );
        Ok(())
    }

    fn rebuild_resolver(&mut self) {
        self.resolver = LinkResolver::build(self.notes.keys());
    }

    /// Returns `content` with every link resolving to `from` retargeted to `to`.
    fn rewrite_links(&self, content: &str, from: &NoteId, to: &NoteId) -> String {
        let mut out = String::with_capacity(content.len());
        let mut last = 0;
        for link in parse_links(content) {
            if self.resolve_link(&link.target) != Some(from) {
                continue;
            }
            out.push_str(&content[last..link.range.start]);
            out.push_str("[[");
            out.push_str(to.as_str());
            if let Some(h) = &link.heading {
                out.push('#');
                out.push_str(h);
            }
            if let Some(a) = &link.alias {
                out.push('|');
                out.push_str(a);
            }
            out.push_str("]]");
            last = link.range.end;
        }
        out.push_str(&content[last..]);
        out
    }
}

/// Lookup tables used to resolve link targets quickly.
#[derive(Debug, Default)]
struct LinkResolver {
    by_path: HashMap<String, NoteId>,
    /// Lower-cased file name -> candidates sorted by path length.
    by_name: HashMap<String, Vec<NoteId>>,
}

impl LinkResolver {
    fn build<'a>(ids: impl Iterator<Item = &'a NoteId>) -> Self {
        let mut resolver = Self::default();
        for id in ids {
            resolver.by_path.insert(id.link_key(), id.clone());
            resolver
                .by_name
                .entry(id.name().to_lowercase())
                .or_default()
                .push(id.clone());
        }
        for candidates in resolver.by_name.values_mut() {
            candidates.sort_by_key(|id| (id.as_str().len(), id.clone()));
        }
        resolver
    }

    fn resolve(&self, target: &str) -> Option<&NoteId> {
        let id = NoteId::new(target).ok()?;
        self.by_path
            .get(&id.link_key())
            .or_else(|| self.by_name.get(&id.name().to_lowercase())?.first())
    }
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with('.'))
}

fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }
    Ok(())
}

fn atomic_write(path: &Path, content: &str) -> Result<()> {
    let tmp = path.with_extension(format!("{NOTE_EXTENSION}.tmp"));
    fs::write(&tmp, content).map_err(|e| Error::io(&tmp, e))?;
    fs::rename(&tmp, path).map_err(|e| Error::io(path, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> NoteId {
        NoteId::new(s).unwrap()
    }

    fn vault() -> (tempfile::TempDir, Vault) {
        let dir = tempfile::tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        (dir, vault)
    }

    #[test]
    fn create_save_reload() {
        let (dir, mut v) = vault();
        v.create(id("Projects/Idea"), "hello").unwrap();
        assert!(dir.path().join("Projects/Idea.md").exists());
        assert!(v.create(id("Projects/Idea"), "again").is_err());

        v.save(&id("Projects/Idea"), "updated").unwrap();
        let reopened = Vault::open(dir.path()).unwrap();
        assert_eq!(reopened.get(&id("Projects/Idea")).unwrap().content, "updated");
    }

    #[test]
    fn skips_hidden_folders() {
        let (dir, _) = vault();
        fs::create_dir_all(dir.path().join(".obsidian")).unwrap();
        fs::write(dir.path().join(".obsidian/x.md"), "").unwrap();
        fs::write(dir.path().join("visible.md"), "").unwrap();
        let v = Vault::open(dir.path()).unwrap();
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn resolves_links_like_obsidian() {
        let (_d, mut v) = vault();
        v.create(id("a/deep/Note"), "").unwrap();
        v.create(id("b/Note"), "").unwrap();
        v.create(id("Other"), "").unwrap();
        assert_eq!(v.resolve_link("note"), Some(&id("b/Note")));
        assert_eq!(v.resolve_link("a/deep/note"), Some(&id("a/deep/Note")));
        assert_eq!(v.resolve_link("other"), Some(&id("Other")));
        assert_eq!(v.resolve_link("missing"), None);
    }

    #[test]
    fn rename_updates_links() {
        let (_d, mut v) = vault();
        v.create(id("Old"), "").unwrap();
        v.create(id("Ref"), "see [[Old]] and [[old#H|alias]] and [[Other]]").unwrap();
        let updated = v.rename(&id("Old"), id("Folder/New")).unwrap();
        assert_eq!(updated, 1);
        assert!(v.get(&id("Old")).is_none());
        assert_eq!(
            v.get(&id("Ref")).unwrap().content,
            "see [[Folder/New]] and [[Folder/New#H|alias]] and [[Other]]"
        );
    }

    #[test]
    fn append_and_delete() {
        let (_d, mut v) = vault();
        v.append(&id("Log"), "one").unwrap();
        v.append(&id("Log"), "two").unwrap();
        assert_eq!(v.get(&id("Log")).unwrap().content, "one\ntwo\n");
        v.delete(&id("Log")).unwrap();
        assert!(v.is_empty());
    }

    #[test]
    fn unique_ids() {
        let (_d, mut v) = vault();
        v.create(id("Note"), "").unwrap();
        v.create(id("Note 2"), "").unwrap();
        assert_eq!(v.unique_id(&id("Note")), id("Note 3"));
        assert_eq!(v.unique_id(&id("Fresh")), id("Fresh"));
    }

    #[test]
    fn search_ranks_titles_first() {
        let (_d, mut v) = vault();
        v.create(id("Alpha"), "mentions rust").unwrap();
        v.create(id("Rust"), "lang").unwrap();
        let hits = v.search("rust");
        assert_eq!(hits[0].id, id("Rust"));
        assert_eq!(hits[1].snippet.as_deref(), Some("mentions rust"));
    }
}
