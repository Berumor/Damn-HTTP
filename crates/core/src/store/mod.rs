//! The workspace store: a workspace as files on disk.
//!
//! [`WorkspaceStore`] creates and opens workspaces, lists their tree, and
//! creates, reads, saves, renames, duplicates, moves and deletes
//! collections, folders and requests. It owns three rules:
//!
//! - **Paths.** Callers name items by workspace-relative paths with `/`
//!   separators, as returned by [`WorkspaceStore::tree`]. A path that would
//!   leave `collections/` is refused, and symbolic links are never followed.
//! - **Names.** File and directory names are a slug of the item's name, with
//!   a numeric suffix on a collision (see `docs/FORMAT.md`).
//! - **Local values.** Saving a request writes the committed part to the
//!   request file and the local part to `.damnhttp/values.yaml` (D-021).
//!   Reading puts them back together.
//!
//! Everything here is synchronous and free of UI concerns. Environments are
//! added in M2.

mod error;
mod fsutil;
mod slug;
mod tree;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::format::{
    self, read_collection, read_folder, read_local_values, read_request, read_workspace,
    write_collection, write_folder, write_local_values, write_request, write_workspace,
};
use crate::model::{
    Auth, Collection, Folder, LocalValues, OrderKey, Request, RequestId, RequestLocalValues,
    Workspace,
};

pub use error::StoreError;
pub use fsutil::GITIGNORE_ENTRY;
pub use slug::slugify;
pub use tree::{Tree, TreeNode, TreeNodeKind, TreeProblem};

/// The directory that holds the collections; also the parent path to pass
/// when creating or moving a collection.
pub const COLLECTIONS_DIR: &str = "collections";

const MANIFEST_FILE: &str = "damnhttp.yaml";
const GITIGNORE_FILE: &str = ".gitignore";
const LOCAL_DIR: &str = ".damnhttp";
const VALUES_FILE: &str = "values.yaml";
const COLLECTION_FILE: &str = "collection.yaml";
const FOLDER_FILE: &str = "folder.yaml";
const REQUEST_EXTENSION: &str = ".yaml";

/// A generated order key longer than this makes the store rewrite the whole
/// sibling group with short keys (D-007).
const MAX_ORDER_KEY_LENGTH: usize = 12;

type Result<T> = std::result::Result<T, StoreError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Collection,
    Folder,
    Request,
}

impl From<&TreeNodeKind> for Kind {
    fn from(kind: &TreeNodeKind) -> Self {
        match kind {
            TreeNodeKind::Collection => Kind::Collection,
            TreeNodeKind::Folder => Kind::Folder,
            TreeNodeKind::Request { .. } => Kind::Request,
        }
    }
}

/// A workspace on disk.
#[derive(Debug, Clone)]
pub struct WorkspaceStore {
    root: PathBuf,
}

impl WorkspaceStore {
    /// Turns `root` into a new workspace: writes `damnhttp.yaml` and makes
    /// sure `.gitignore` ignores the local-state directory. The directory is
    /// created if missing and may already hold other files (for example a
    /// freshly cloned repository).
    pub fn create(root: &Path, name: &str) -> Result<Self> {
        let name = clean_name(name)?;
        fs::create_dir_all(root).map_err(io_error("."))?;
        let root = fs::canonicalize(root).map_err(io_error("."))?;
        if root.join(MANIFEST_FILE).exists() {
            return Err(StoreError::AlreadyAWorkspace);
        }
        let store = Self { root };
        store.save_workspace(&Workspace { name })?;
        store.ensure_gitignore()?;
        Ok(store)
    }

    /// Opens an existing workspace. Fails with [`StoreError::NotAWorkspace`]
    /// if `root` has no `damnhttp.yaml`, and with a format error if the
    /// manifest is unreadable or from a newer version of the app.
    pub fn open(root: &Path) -> Result<Self> {
        let Ok(root) = fs::canonicalize(root) else {
            return Err(StoreError::NotAWorkspace);
        };
        if !root.join(MANIFEST_FILE).is_file() {
            return Err(StoreError::NotAWorkspace);
        }
        let store = Self { root };
        store.workspace()?;
        store.ensure_gitignore()?;
        Ok(store)
    }

    /// The workspace directory, as an absolute path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn workspace(&self) -> Result<Workspace> {
        let text = self.read_text(MANIFEST_FILE)?;
        read_workspace(&text).map_err(format_error(MANIFEST_FILE))
    }

    pub fn save_workspace(&self, workspace: &Workspace) -> Result<()> {
        self.write_text(MANIFEST_FILE, &write_workspace(workspace))
    }

    /// Reads the whole tree. Unreadable files are reported in
    /// [`Tree::problems`] instead of failing the call.
    pub fn tree(&self) -> Result<Tree> {
        let mut problems = Vec::new();
        let collections = self.load_children(COLLECTIONS_DIR, &mut problems, true)?;
        Ok(Tree {
            collections,
            problems,
        })
    }

    // ----- creating -----

    /// Creates an empty collection after the existing ones and returns its path.
    pub fn create_collection(&self, name: &str) -> Result<String> {
        let name = clean_name(name)?;
        fs::create_dir_all(self.abs(COLLECTIONS_DIR)).map_err(io_error(COLLECTIONS_DIR))?;
        let order = self.order_for_insert(COLLECTIONS_DIR, None, usize::MAX)?;
        let path = self.new_directory(COLLECTIONS_DIR, &name)?;
        let collection = Collection {
            name,
            order,
            description: String::new(),
            variables: Vec::new(),
            auth: Auth::None,
        };
        self.write_text(
            &join(&path, COLLECTION_FILE),
            &write_collection(&collection),
        )?;
        Ok(path)
    }

    /// Creates an empty folder at the end of `parent` (a collection or
    /// folder) and returns its path.
    pub fn create_folder(&self, parent: &str, name: &str) -> Result<String> {
        let name = clean_name(name)?;
        self.expect_container(parent)?;
        let order = self.order_for_insert(parent, None, usize::MAX)?;
        let path = self.new_directory(parent, &name)?;
        let folder = Folder {
            name,
            order,
            description: String::new(),
            auth: Auth::Inherit,
        };
        self.write_text(&join(&path, FOLDER_FILE), &write_folder(&folder))?;
        Ok(path)
    }

    /// Creates an empty `GET` request at the end of `parent` (a collection
    /// or folder) and returns its path.
    pub fn create_request(&self, parent: &str, name: &str) -> Result<String> {
        let name = clean_name(name)?;
        self.expect_container(parent)?;
        let order = self.order_for_insert(parent, None, usize::MAX)?;
        let request = Request::new(name, order);
        let path = self.new_request_path(parent, &request.name)?;
        self.store_request(&path, request)?;
        Ok(path)
    }

    // ----- reading and saving -----

    /// Reads a request as the editor shows it: the committed file merged
    /// with this machine's local values.
    ///
    /// A literal query or path variable value found in the file (a hand
    /// edit) is moved to the local values; the next save writes the file
    /// without it.
    pub fn read_request(&self, path: &str) -> Result<Request> {
        self.expect_kind(path, Kind::Request)?;
        self.load_request(path)
    }

    /// Saves a request: the committed part to its file, the local part to
    /// `.damnhttp/values.yaml`. The file keeps its name; use
    /// [`WorkspaceStore::rename`] to rename.
    pub fn save_request(&self, path: &str, request: Request) -> Result<()> {
        self.expect_kind(path, Kind::Request)?;
        // The id never changes once the file exists (D-022). An unreadable
        // file has no id to protect and is simply replaced.
        if let Ok(text) = self.read_text(path)
            && let Ok(file) = read_request(&text)
            && file.request.as_request().id != request.id
        {
            return Err(StoreError::IdMismatch(path.to_owned()));
        }
        self.store_request(path, request)
    }

    pub fn read_collection(&self, path: &str) -> Result<Collection> {
        self.expect_kind(path, Kind::Collection)?;
        self.load_collection(path)
    }

    /// Saves a collection's own settings. The directory keeps its name.
    pub fn save_collection(&self, path: &str, collection: &Collection) -> Result<()> {
        self.expect_kind(path, Kind::Collection)?;
        self.write_text(&join(path, COLLECTION_FILE), &write_collection(collection))
    }

    pub fn read_folder(&self, path: &str) -> Result<Folder> {
        self.expect_kind(path, Kind::Folder)?;
        self.load_folder(path)
    }

    /// Saves a folder's own settings. The directory keeps its name.
    pub fn save_folder(&self, path: &str, folder: &Folder) -> Result<()> {
        self.expect_kind(path, Kind::Folder)?;
        self.write_text(&join(path, FOLDER_FILE), &write_folder(folder))
    }

    // ----- tree operations -----

    /// Renames an item: updates `name` in its file and renames the file or
    /// directory to the matching slug. Returns the new path, which is the
    /// old one when the slug did not change.
    pub fn rename(&self, path: &str, new_name: &str) -> Result<String> {
        let name = clean_name(new_name)?;
        let kind = self.kind_of(path)?;
        match kind {
            Kind::Request => {
                let mut request = self.load_request(path)?;
                request.name = name.clone();
                self.store_request(path, request)?;
            }
            Kind::Collection => {
                let mut collection = self.load_collection(path)?;
                collection.name = name.clone();
                self.write_text(&join(path, COLLECTION_FILE), &write_collection(&collection))?;
            }
            Kind::Folder => {
                let mut folder = self.load_folder(path)?;
                folder.name = name.clone();
                self.write_text(&join(path, FOLDER_FILE), &write_folder(&folder))?;
            }
        }

        let (parent, file_name) = split_parent(path);
        let taken = self.taken_stems(parent, Some(file_name))?;
        let stem = slug::unique_stem(&slugify(&name), &taken);
        let new_file_name = match kind {
            Kind::Request => format!("{stem}{REQUEST_EXTENSION}"),
            Kind::Collection | Kind::Folder => stem,
        };
        if new_file_name == file_name {
            return Ok(path.to_owned());
        }
        let new_path = join(parent, &new_file_name);
        fs::rename(self.abs(path), self.abs(&new_path)).map_err(io_error(path))?;
        Ok(new_path)
    }

    /// Copies an item, placing the copy right after the original with
    /// " copy" added to its name. Every copied request gets a new id and a
    /// copy of the original's local values (D-022). Returns the copy's path.
    pub fn duplicate(&self, path: &str) -> Result<String> {
        let kind = self.kind_of(path)?;
        let (parent, _) = split_parent(path);
        let mut problems = Vec::new();
        let siblings = self.load_children(parent, &mut problems, false)?;
        let position = siblings
            .iter()
            .position(|node| node.path == path)
            .map_or(usize::MAX, |index| index + 1);
        let order = self.order_for_insert(parent, None, position)?;
        let name = match kind {
            Kind::Request => self.load_request(path)?.name,
            Kind::Collection => self.load_collection(path)?.name,
            Kind::Folder => self.load_folder(path)?.name,
        };
        self.copy_node(path, kind, parent, Some(format!("{name} copy")), order)
    }

    /// Moves an item to `position` among the children of `new_parent`, and
    /// returns its new path.
    ///
    /// `position` counts the children of `new_parent` without the item
    /// itself; anything past the end means "last". Use the same parent to
    /// reorder. Pass [`COLLECTIONS_DIR`] as the parent to reorder
    /// collections. Only the moved item's `order` key is rewritten, unless
    /// the group's keys have to be renormalized.
    pub fn move_to(&self, path: &str, new_parent: &str, position: usize) -> Result<String> {
        let kind = self.kind_of(path)?;
        let (parent, file_name) = split_parent(path);
        match kind {
            Kind::Collection => {
                if new_parent != COLLECTIONS_DIR {
                    return Err(StoreError::InvalidMove(
                        "a collection cannot be placed inside another item",
                    ));
                }
            }
            Kind::Folder | Kind::Request => {
                if new_parent == COLLECTIONS_DIR {
                    return Err(StoreError::InvalidMove(
                        "folders and requests must be inside a collection",
                    ));
                }
                self.expect_container(new_parent)?;
            }
        }
        if kind == Kind::Folder
            && (new_parent == path || new_parent.starts_with(&format!("{path}/")))
        {
            return Err(StoreError::InvalidMove(
                "a folder cannot be moved into itself",
            ));
        }

        let order = self.order_for_insert(new_parent, Some(path), position)?;
        self.set_order(path, kind, order)?;
        if new_parent == parent {
            return Ok(path.to_owned());
        }

        let stem = match kind {
            Kind::Request => file_name
                .strip_suffix(REQUEST_EXTENSION)
                .unwrap_or(file_name),
            Kind::Collection | Kind::Folder => file_name,
        };
        let stem = slug::unique_stem(stem, &self.taken_stems(new_parent, None)?);
        let new_file_name = match kind {
            Kind::Request => format!("{stem}{REQUEST_EXTENSION}"),
            Kind::Collection | Kind::Folder => stem,
        };
        let new_path = join(new_parent, &new_file_name);
        fs::rename(self.abs(path), self.abs(&new_path)).map_err(io_error(path))?;
        Ok(new_path)
    }

    /// Deletes a request, or a folder or collection with everything in it.
    ///
    /// Local values of deleted requests are kept, so restoring the files
    /// (for example with git) brings the values back. Use
    /// [`WorkspaceStore::clean_local_values`] to drop them.
    pub fn delete(&self, path: &str) -> Result<()> {
        match self.kind_of(path)? {
            Kind::Request => fs::remove_file(self.abs(path)),
            Kind::Collection | Kind::Folder => fs::remove_dir_all(self.abs(path)),
        }
        .map_err(io_error(path))
    }

    // ----- local data and repairs -----

    /// "Clear unused local data": removes local values whose request no
    /// longer exists, and returns how many requests' values were removed.
    ///
    /// Refuses to run while any file is unreadable, because the id inside
    /// an unreadable request file is unknown and its values would be lost.
    pub fn clean_local_values(&self) -> Result<usize> {
        let tree = self.tree()?;
        if !tree.problems.is_empty() {
            return Err(StoreError::UnreadableFiles(tree.problems.len()));
        }
        let live: HashSet<RequestId> = tree.requests().into_iter().map(|(_, id)| id).collect();
        let mut values = self.load_values()?;
        let before = values.requests.len();
        values.requests.retain(|id, _| live.contains(id));
        let removed = before - values.requests.len();
        if removed > 0 {
            self.save_values(&values)?;
        }
        Ok(removed)
    }

    /// Gives a new id to request files that share an id with another file
    /// (a file copied by hand, D-022), and returns the paths that changed.
    ///
    /// The file whose path sorts first keeps the id. Each copy keeps a copy
    /// of the local values. Choosing by git state (the untracked or newer
    /// file is the copy) is added with the git engine in M3.
    pub fn repair_duplicate_ids(&self) -> Result<Vec<String>> {
        let tree = self.tree()?;
        let mut by_id: BTreeMap<RequestId, Vec<&str>> = BTreeMap::new();
        for (path, id) in tree.requests() {
            by_id.entry(id).or_default().push(path);
        }
        let mut repaired = Vec::new();
        for paths in by_id.values_mut() {
            paths.sort_unstable();
            for path in paths.iter().skip(1) {
                let mut request = self.load_request(path)?;
                request.id = RequestId::generate();
                self.store_request(path, request)?;
                repaired.push((*path).to_owned());
            }
        }
        repaired.sort_unstable();
        Ok(repaired)
    }

    // ----- internals: paths -----

    fn abs(&self, path: &str) -> PathBuf {
        let mut abs = self.root.clone();
        abs.extend(path.split('/'));
        abs
    }

    /// The kind of the item at `path`, after checking the path stays inside
    /// `collections/` and is not a symbolic link.
    fn kind_of(&self, path: &str) -> Result<Kind> {
        check_path(path)?;
        let abs = self.abs(path);
        let Ok(metadata) = fs::symlink_metadata(&abs) else {
            return Err(StoreError::NotFound(path.to_owned()));
        };
        let depth = path.split('/').count();
        let (_, file_name) = split_parent(path);
        if metadata.is_dir() {
            if depth == 2 && abs.join(COLLECTION_FILE).is_file() {
                return Ok(Kind::Collection);
            }
            if depth > 2 && abs.join(FOLDER_FILE).is_file() {
                return Ok(Kind::Folder);
            }
        } else if metadata.is_file() && depth > 2 && is_request_file_name(file_name) {
            return Ok(Kind::Request);
        }
        Err(StoreError::WrongKind {
            path: path.to_owned(),
            expected: "collection, folder or request",
        })
    }

    fn expect_kind(&self, path: &str, expected: Kind) -> Result<()> {
        if self.kind_of(path)? == expected {
            Ok(())
        } else {
            Err(StoreError::WrongKind {
                path: path.to_owned(),
                expected: match expected {
                    Kind::Collection => "collection",
                    Kind::Folder => "folder",
                    Kind::Request => "request",
                },
            })
        }
    }

    fn expect_container(&self, path: &str) -> Result<()> {
        match self.kind_of(path)? {
            Kind::Collection | Kind::Folder => Ok(()),
            Kind::Request => Err(StoreError::WrongKind {
                path: path.to_owned(),
                expected: "collection or folder",
            }),
        }
    }

    // ----- internals: files -----

    fn read_text(&self, path: &str) -> Result<String> {
        fs::read_to_string(self.abs(path)).map_err(io_error(path))
    }

    fn write_text(&self, path: &str, contents: &str) -> Result<()> {
        fsutil::write_atomic(&self.abs(path), contents).map_err(io_error(path))
    }

    fn ensure_gitignore(&self) -> Result<()> {
        let existing = match fs::read_to_string(self.abs(GITIGNORE_FILE)) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(io_error(GITIGNORE_FILE)(error)),
        };
        match fsutil::gitignore_with_entry(&existing) {
            Some(updated) => self.write_text(GITIGNORE_FILE, &updated),
            None => Ok(()),
        }
    }

    fn load_collection(&self, path: &str) -> Result<Collection> {
        let file = join(path, COLLECTION_FILE);
        read_collection(&self.read_text(&file)?).map_err(format_error(&file))
    }

    fn load_folder(&self, path: &str) -> Result<Folder> {
        let file = join(path, FOLDER_FILE);
        read_folder(&self.read_text(&file)?).map_err(format_error(&file))
    }

    fn values_path() -> String {
        join(LOCAL_DIR, VALUES_FILE)
    }

    fn load_values(&self) -> Result<LocalValues> {
        let path = Self::values_path();
        match fs::read_to_string(self.abs(&path)) {
            Ok(text) => read_local_values(&text).map_err(format_error(&path)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(LocalValues::default()),
            Err(error) => Err(io_error(&path)(error)),
        }
    }

    fn save_values(&self, values: &LocalValues) -> Result<()> {
        fs::create_dir_all(self.abs(LOCAL_DIR)).map_err(io_error(LOCAL_DIR))?;
        self.write_text(&Self::values_path(), &write_local_values(values))
    }

    /// Reads a request file and merges this machine's local values into it.
    fn load_request(&self, path: &str) -> Result<Request> {
        let file = read_request(&self.read_text(path)?).map_err(format_error(path))?;
        let id = file.request.as_request().id;
        let mut values = self.load_values()?;
        let mut local = values.requests.get(&id).cloned().unwrap_or_default();
        if !file.literals.is_empty() {
            // Literals typed into the file by hand: keep them locally. A
            // value already stored on this machine wins over the file's.
            absorb_literals(&mut local, file.literals);
            values.requests.insert(id, local.clone());
            self.save_values(&values)?;
        }
        let mut request = file.request.merge(&local);
        // One entry per variable in the URL, in URL order; entries the URL
        // no longer has stay visible as orphans until the next save (D-028).
        request.align_path_params(true);
        Ok(request)
    }

    /// Splits a request and writes both parts. No kind or id checks.
    fn store_request(&self, path: &str, mut request: Request) -> Result<()> {
        let id = request.id;
        // Saving drops path variable entries the URL no longer has (D-028).
        request.move_query_out_of_url();
        request.align_path_params(false);
        let (committed, local) = request.split();
        // Read the local values first: if they are unreadable, nothing is
        // written and the editor still holds what the user typed.
        let mut values = self.load_values()?;
        self.write_text(path, &write_request(&committed))?;
        let changed = if local.is_empty() {
            values.requests.remove(&id).is_some()
        } else if values.requests.get(&id) == Some(&local) {
            false
        } else {
            values.requests.insert(id, local);
            true
        };
        if changed {
            self.save_values(&values)?;
        }
        Ok(())
    }

    // ----- internals: names -----

    /// The stems already used in `dir`, lower-cased: file names without
    /// `.yaml`, and directory names. `except` is a file name to leave out.
    fn taken_stems(&self, dir: &str, except: Option<&str>) -> Result<Vec<String>> {
        let mut taken = Vec::new();
        for entry in fs::read_dir(self.abs(dir)).map_err(io_error(dir))? {
            let entry = entry.map_err(io_error(dir))?;
            let Some(file_name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if Some(file_name.as_str()) == except {
                continue;
            }
            let lower = file_name.to_lowercase();
            taken.push(match lower.strip_suffix(REQUEST_EXTENSION) {
                Some(stem) => stem.to_owned(),
                None => lower,
            });
        }
        Ok(taken)
    }

    fn new_request_path(&self, parent: &str, name: &str) -> Result<String> {
        let stem = slug::unique_stem(&slugify(name), &self.taken_stems(parent, None)?);
        Ok(join(parent, &format!("{stem}{REQUEST_EXTENSION}")))
    }

    fn new_directory(&self, parent: &str, name: &str) -> Result<String> {
        let stem = slug::unique_stem(&slugify(name), &self.taken_stems(parent, None)?);
        let path = join(parent, &stem);
        fs::create_dir(self.abs(&path)).map_err(io_error(&path))?;
        Ok(path)
    }

    // ----- internals: tree -----

    /// The children of `parent` in display order. With `recursive`, their
    /// children too. Unreadable files go to `problems`.
    fn load_children(
        &self,
        parent: &str,
        problems: &mut Vec<TreeProblem>,
        recursive: bool,
    ) -> Result<Vec<TreeNode>> {
        let entries = match fs::read_dir(self.abs(parent)) {
            Ok(entries) => entries,
            // A new workspace has no `collections/` directory yet.
            Err(error) if error.kind() == io::ErrorKind::NotFound && parent == COLLECTIONS_DIR => {
                return Ok(Vec::new());
            }
            Err(error) => return Err(io_error(parent)(error)),
        };
        let at_root = parent == COLLECTIONS_DIR;
        let mut nodes = Vec::new();
        for entry in entries {
            let entry = entry.map_err(io_error(parent))?;
            let Some(file_name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if file_name.starts_with('.') {
                continue;
            }
            // `file_type` does not follow symbolic links, so links are
            // neither files nor directories here and are skipped.
            let file_type = entry.file_type().map_err(io_error(parent))?;
            let path = join(parent, &file_name);
            let loaded = if file_type.is_dir() {
                let marker = if at_root {
                    COLLECTION_FILE
                } else {
                    FOLDER_FILE
                };
                if !entry.path().join(marker).is_file() {
                    continue;
                }
                if at_root {
                    self.load_collection(&path)
                        .map(|c| (c.name, c.order, TreeNodeKind::Collection))
                } else {
                    self.load_folder(&path)
                        .map(|f| (f.name, f.order, TreeNodeKind::Folder))
                }
            } else if file_type.is_file() && !at_root && is_request_file_name(&file_name) {
                self.read_text(&path).and_then(|text| {
                    let file = read_request(&text).map_err(format_error(&path))?;
                    let request = file.request.as_request();
                    Ok((
                        request.name.clone(),
                        request.order.clone(),
                        TreeNodeKind::Request {
                            id: request.id,
                            method: request.method.clone(),
                        },
                    ))
                })
            } else {
                continue;
            };
            match loaded {
                Ok((name, order, kind)) => {
                    let children = if recursive && file_type.is_dir() {
                        self.load_children(&path, problems, true)?
                    } else {
                        Vec::new()
                    };
                    nodes.push(TreeNode {
                        path,
                        name,
                        order,
                        kind,
                        children,
                    });
                }
                Err(error) => problems.push(TreeProblem {
                    message: problem_message(&error),
                    path,
                }),
            }
        }
        nodes.sort_by(|a, b| a.order.cmp(&b.order).then_with(|| a.path.cmp(&b.path)));
        Ok(nodes)
    }

    // ----- internals: ordering -----

    /// An order key that puts a new child at `position` in `parent`.
    /// `exclude` is the path of an item to ignore (the one being moved).
    ///
    /// When no key fits between the neighbours, or the key would be too
    /// long, every sibling gets a fresh short key.
    fn order_for_insert(
        &self,
        parent: &str,
        exclude: Option<&str>,
        position: usize,
    ) -> Result<OrderKey> {
        let mut problems = Vec::new();
        let mut siblings = self.load_children(parent, &mut problems, false)?;
        siblings.retain(|node| Some(node.path.as_str()) != exclude);
        let position = position.min(siblings.len());
        let before = position
            .checked_sub(1)
            .and_then(|index| siblings.get(index))
            .map(|node| &node.order);
        let after = siblings.get(position).map(|node| &node.order);
        if let Some(key) = OrderKey::between(before, after)
            && key.as_str().len() <= MAX_ORDER_KEY_LENGTH
        {
            return Ok(key);
        }

        let mut keys = OrderKey::sequence(siblings.len() + 1).into_iter();
        let mut new_key = OrderKey::first();
        let mut existing = siblings.iter();
        for slot in 0..=siblings.len() {
            let Some(key) = keys.next() else { break };
            if slot == position {
                new_key = key;
            } else if let Some(sibling) = existing.next()
                && sibling.order != key
            {
                self.set_order(&sibling.path, Kind::from(&sibling.kind), key)?;
            }
        }
        Ok(new_key)
    }

    fn set_order(&self, path: &str, kind: Kind, order: OrderKey) -> Result<()> {
        match kind {
            Kind::Request => {
                let mut request = self.load_request(path)?;
                request.order = order;
                self.store_request(path, request)
            }
            Kind::Collection => {
                let mut collection = self.load_collection(path)?;
                collection.order = order;
                self.write_text(&join(path, COLLECTION_FILE), &write_collection(&collection))
            }
            Kind::Folder => {
                let mut folder = self.load_folder(path)?;
                folder.order = order;
                self.write_text(&join(path, FOLDER_FILE), &write_folder(&folder))
            }
        }
    }

    // ----- internals: copying -----

    /// Copies `source` into `target_parent`. `name` replaces the name of the
    /// top item; children keep theirs. Requests get new ids.
    fn copy_node(
        &self,
        source: &str,
        kind: Kind,
        target_parent: &str,
        name: Option<String>,
        order: OrderKey,
    ) -> Result<String> {
        let target = match kind {
            Kind::Request => {
                let mut request = self.load_request(source)?;
                request.id = RequestId::generate();
                request.order = order;
                if let Some(name) = name {
                    request.name = name;
                }
                let target = self.new_request_path(target_parent, &request.name)?;
                self.store_request(&target, request)?;
                return Ok(target);
            }
            Kind::Collection => {
                let mut collection = self.load_collection(source)?;
                collection.order = order;
                if let Some(name) = name {
                    collection.name = name;
                }
                let target = self.new_directory(target_parent, &collection.name)?;
                self.write_text(
                    &join(&target, COLLECTION_FILE),
                    &write_collection(&collection),
                )?;
                target
            }
            Kind::Folder => {
                let mut folder = self.load_folder(source)?;
                folder.order = order;
                if let Some(name) = name {
                    folder.name = name;
                }
                let target = self.new_directory(target_parent, &folder.name)?;
                self.write_text(&join(&target, FOLDER_FILE), &write_folder(&folder))?;
                target
            }
        };

        let copied = (|| {
            let mut problems = Vec::new();
            let children = self.load_children(source, &mut problems, false)?;
            if !problems.is_empty() {
                return Err(StoreError::UnreadableFiles(problems.len()));
            }
            for child in children {
                self.copy_node(
                    &child.path,
                    Kind::from(&child.kind),
                    &target,
                    None,
                    child.order,
                )?;
            }
            Ok(())
        })();
        if let Err(error) = copied {
            // Do not leave half a copy behind; the original error is the
            // one worth reporting.
            let _ = fs::remove_dir_all(self.abs(&target));
            return Err(error);
        }
        Ok(target)
    }
}

/// Checks that `path` is a workspace-relative path under `collections/`
/// made of plain components: no `.`, `..`, hidden names, backslashes,
/// drive letters or empty parts.
fn check_path(path: &str) -> Result<()> {
    let valid = path.split('/').enumerate().all(|(index, part)| {
        !part.is_empty()
            && !part.starts_with('.')
            && !part.contains(['\\', ':', '\0'])
            && (index > 0 || part == COLLECTIONS_DIR)
    });
    if valid {
        Ok(())
    } else {
        Err(StoreError::InvalidPath(path.to_owned()))
    }
}

fn is_request_file_name(file_name: &str) -> bool {
    file_name.ends_with(REQUEST_EXTENSION)
        && file_name != COLLECTION_FILE
        && file_name != FOLDER_FILE
}

fn join(parent: &str, name: &str) -> String {
    format!("{parent}/{name}")
}

/// Splits `a/b/c` into (`a/b`, `c`). A single component has an empty parent.
fn split_parent(path: &str) -> (&str, &str) {
    path.rsplit_once('/').unwrap_or(("", path))
}

fn clean_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        Err(StoreError::EmptyName)
    } else {
        Ok(name.to_owned())
    }
}

/// Adds literals found in a request file to the local values, without
/// replacing anything already stored.
fn absorb_literals(local: &mut RequestLocalValues, literals: RequestLocalValues) {
    for (name, value) in literals.path_params {
        local.path_params.entry(name).or_insert(value);
    }
    for literal in literals.query {
        let known = local
            .query
            .iter()
            .any(|l| l.name == literal.name && l.occurrence == literal.occurrence);
        if !known {
            local.query.push(literal);
        }
    }
    local.skip_tls_verify |= literals.skip_tls_verify;
}

fn problem_message(error: &StoreError) -> String {
    match error {
        // The path is already carried by the `TreeProblem`.
        StoreError::Format { error, .. } => error.to_string(),
        StoreError::Io { source, .. } => source.to_string(),
        other => other.to_string(),
    }
}

fn io_error(path: &str) -> impl FnOnce(io::Error) -> StoreError + use<> {
    let path = path.to_owned();
    move |source| StoreError::Io { path, source }
}

fn format_error(path: &str) -> impl FnOnce(format::FormatError) -> StoreError + use<> {
    let path = path.to_owned();
    move |error| StoreError::Format { path, error }
}
