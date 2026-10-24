//! Link graph: forward links, backlinks and unresolved links.
//!
//! The index is cheap to rebuild from scratch (all content is already in
//! memory), so the app simply calls [`LinkIndex::build`] after every change.
//! This keeps the logic trivially correct; it can be made incremental later
//! if vaults get very large.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::links::{parse_links, WikiLink};
use crate::note::NoteId;
use crate::vault::Vault;

/// A note that links to another one, with the line containing the link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backlink {
    pub source: NoteId,
    /// Trimmed line of the source note containing the link (for context).
    pub context: String,
}

/// A link found in a note, together with what it resolves to.
#[derive(Debug, Clone)]
pub struct OutgoingLink {
    pub link: WikiLink,
    pub resolved: Option<NoteId>,
}

#[derive(Debug, Default)]
pub struct LinkIndex {
    outgoing: HashMap<NoteId, Vec<OutgoingLink>>,
    backlinks: HashMap<NoteId, Vec<Backlink>>,
    /// Link target text -> notes containing that dangling link.
    unresolved: BTreeMap<String, BTreeSet<NoteId>>,
}

impl LinkIndex {
    pub fn build(vault: &Vault) -> Self {
        let mut index = Self::default();

        for note in vault.notes() {
            let body = note.content.as_str();
            let mut out = Vec::new();
            // A note linking twice to the same target yields one backlink.
            let mut seen_targets = BTreeSet::new();

            for link in parse_links(body) {
                let resolved = vault.resolve_link(&link.target).cloned();
                match &resolved {
                    Some(target) if seen_targets.insert(target.clone()) => {
                        index.backlinks.entry(target.clone()).or_default().push(Backlink {
                            source: note.id.clone(),
                            context: line_around(body, link.range.start).to_owned(),
                        });
                    }
                    Some(_) => {}
                    None => {
                        index
                            .unresolved
                            .entry(link.target.clone())
                            .or_default()
                            .insert(note.id.clone());
                    }
                }
                out.push(OutgoingLink { link, resolved });
            }
            index.outgoing.insert(note.id.clone(), out);
        }
        index
    }

    /// Notes linking to `id`, sorted by source id.
    pub fn backlinks(&self, id: &NoteId) -> &[Backlink] {
        self.backlinks.get(id).map(Vec::as_slice).unwrap_or_default()
    }

    /// Links contained in `id`, in document order.
    pub fn outgoing(&self, id: &NoteId) -> &[OutgoingLink] {
        self.outgoing.get(id).map(Vec::as_slice).unwrap_or_default()
    }

    /// Dangling link targets and the notes that reference them.
    pub fn unresolved(&self) -> &BTreeMap<String, BTreeSet<NoteId>> {
        &self.unresolved
    }

    /// Notes with neither incoming nor outgoing resolved links.
    pub fn orphans<'a>(&'a self, vault: &'a Vault) -> impl Iterator<Item = &'a NoteId> + 'a {
        vault.notes().map(|n| &n.id).filter(|id| {
            self.backlinks(id).is_empty() && self.outgoing(id).iter().all(|l| l.resolved.is_none())
        })
    }
}

fn line_around(text: &str, byte: usize) -> &str {
    let start = text[..byte].rfind('\n').map_or(0, |i| i + 1);
    let end = text[byte..].find('\n').map_or(text.len(), |i| byte + i);
    text[start..end].trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> NoteId {
        NoteId::new(s).unwrap()
    }

    #[test]
    fn builds_graph() {
        let dir = tempfile::tempdir().unwrap();
        let mut v = Vault::open(dir.path()).unwrap();
        v.create(id("A"), "links to [[B]] twice [[b]]\nand [[Ghost]]").unwrap();
        v.create(id("B"), "back to [[A]]").unwrap();
        v.create(id("Lonely"), "nothing").unwrap();

        let idx = LinkIndex::build(&v);
        assert_eq!(idx.backlinks(&id("B")).len(), 1);
        assert_eq!(idx.backlinks(&id("B"))[0].source, id("A"));
        assert_eq!(idx.backlinks(&id("B"))[0].context, "links to [[B]] twice [[b]]");
        assert_eq!(idx.outgoing(&id("A")).len(), 3);
        assert!(idx.unresolved().contains_key("Ghost"));
        assert_eq!(idx.orphans(&v).collect::<Vec<_>>(), vec![&id("Lonely")]);
    }
}
