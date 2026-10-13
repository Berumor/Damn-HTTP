//! What the commands do, without Tauri: each function takes the app state
//! and plain values, so it can be tested on a temporary directory.

use std::path::Path;

use damnhttp_core::store::{COLLECTIONS_DIR, WorkspaceStore};

use crate::dto::{RecentWorkspace, TreeDto, WorkspaceInfo};
use crate::error::AppError;
use crate::state::AppState;

fn info(store: &WorkspaceStore) -> Result<WorkspaceInfo, AppError> {
    Ok(WorkspaceInfo {
        name: store.workspace()?.name,
        path: store.root().to_string_lossy().into_owned(),
    })
}

/// Makes `store` the open workspace and records it as recent.
fn adopt(state: &AppState, store: WorkspaceStore) -> Result<WorkspaceInfo, AppError> {
    let info = info(&store)?;
    state.remember(&info.name, store.root());
    state.allow(store.root());
    state.set_workspace(Some(store));
    Ok(info)
}

pub fn workspace_create(
    state: &AppState,
    folder: &str,
    name: &str,
) -> Result<WorkspaceInfo, AppError> {
    let folder = Path::new(folder);
    if !state.is_allowed(folder) {
        return Err(AppError::folder_not_allowed());
    }
    adopt(state, WorkspaceStore::create(folder, name)?)
}

pub fn workspace_open(state: &AppState, folder: &str) -> Result<WorkspaceInfo, AppError> {
    let folder = Path::new(folder);
    if !state.is_allowed(folder) {
        return Err(AppError::folder_not_allowed());
    }
    let store = WorkspaceStore::open(folder)?;
    // Files copied by hand may share an id; give the copies their own
    // before anything keys local values by it (D-022).
    store.repair_duplicate_ids()?;
    adopt(state, store)
}

pub fn workspace_close(state: &AppState) {
    state.set_workspace(None);
}

pub fn workspace_current(state: &AppState) -> Result<Option<WorkspaceInfo>, AppError> {
    match state.with_workspace(info) {
        Ok(info) => Ok(Some(info)),
        Err(error) if error.code == "NO_WORKSPACE_OPEN" => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn workspace_recent(state: &AppState) -> Vec<RecentWorkspace> {
    state.recent()
}

pub fn workspace_tree(state: &AppState) -> Result<TreeDto, AppError> {
    state.with_workspace(|store| Ok(store.tree()?.into()))
}

pub fn collection_create(state: &AppState, name: &str) -> Result<String, AppError> {
    state.with_workspace(|store| Ok(store.create_collection(name)?))
}

pub fn folder_create(state: &AppState, parent: &str, name: &str) -> Result<String, AppError> {
    state.with_workspace(|store| Ok(store.create_folder(parent, name)?))
}

pub fn request_create(state: &AppState, parent: &str, name: &str) -> Result<String, AppError> {
    state.with_workspace(|store| Ok(store.create_request(parent, name)?))
}

pub fn node_rename(state: &AppState, path: &str, name: &str) -> Result<String, AppError> {
    state.with_workspace(|store| Ok(store.rename(path, name)?))
}

pub fn node_duplicate(state: &AppState, path: &str) -> Result<String, AppError> {
    state.with_workspace(|store| Ok(store.duplicate(path)?))
}

/// `new_parent` is a collection or folder path, or `None` to reorder
/// collections. `position` counts the parent's children without the moved
/// item.
pub fn node_move(
    state: &AppState,
    path: &str,
    new_parent: Option<&str>,
    position: u32,
) -> Result<String, AppError> {
    let parent = new_parent.unwrap_or(COLLECTIONS_DIR);
    let position = usize::try_from(position).unwrap_or(usize::MAX);
    state.with_workspace(|store| Ok(store.move_to(path, parent, position)?))
}

pub fn node_delete(state: &AppState, path: &str) -> Result<(), AppError> {
    state.with_workspace(|store| Ok(store.delete(path)?))
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::dto::NodeKind;

    struct Fixture {
        _data: TempDir,
        folders: TempDir,
        state: AppState,
    }

    fn fixture() -> Fixture {
        let data = TempDir::new().unwrap();
        let state = AppState::new(data.path().to_path_buf());
        Fixture {
            _data: data,
            folders: TempDir::new().unwrap(),
            state,
        }
    }

    impl Fixture {
        /// A folder the user "picked" in the dialog.
        fn picked(&self, name: &str) -> String {
            let folder = self.folders.path().join(name);
            std::fs::create_dir_all(&folder).unwrap();
            self.state.allow(&folder);
            folder.to_string_lossy().into_owned()
        }
    }

    #[test]
    fn nothing_works_without_an_open_workspace() {
        let f = fixture();
        assert_eq!(workspace_current(&f.state).unwrap(), None);
        assert_eq!(
            workspace_tree(&f.state).unwrap_err().code,
            "NO_WORKSPACE_OPEN"
        );
        assert_eq!(
            collection_create(&f.state, "X").unwrap_err().code,
            "NO_WORKSPACE_OPEN"
        );
    }

    #[test]
    fn only_folders_the_user_chose_can_be_opened_or_created() {
        let f = fixture();
        let not_picked = f.folders.path().join("not-picked");
        std::fs::create_dir_all(&not_picked).unwrap();
        let not_picked = not_picked.to_string_lossy().into_owned();

        assert_eq!(
            workspace_create(&f.state, &not_picked, "X")
                .unwrap_err()
                .code,
            "FOLDER_NOT_ALLOWED"
        );
        assert_eq!(
            workspace_open(&f.state, &not_picked).unwrap_err().code,
            "FOLDER_NOT_ALLOWED"
        );
        assert_eq!(
            workspace_open(&f.state, "/").unwrap_err().code,
            "FOLDER_NOT_ALLOWED"
        );
        assert!(
            !std::path::Path::new(&not_picked)
                .join("damnhttp.yaml")
                .exists()
        );

        // The same folder reached through `..` is still the picked one.
        let picked = f.picked("ws");
        let roundabout = format!("{picked}/../ws");
        assert!(workspace_create(&f.state, &roundabout, "Acme").is_ok());
    }

    #[test]
    fn create_open_close_and_the_recent_list() {
        let f = fixture();
        let first = f.picked("first");
        let second = f.picked("second");

        let info = workspace_create(&f.state, &first, "First").unwrap();
        assert_eq!(info.name, "First");
        assert_eq!(workspace_current(&f.state).unwrap(), Some(info.clone()));
        assert_eq!(
            workspace_create(&f.state, &first, "Again")
                .unwrap_err()
                .code,
            "ALREADY_A_WORKSPACE"
        );
        assert_eq!(
            workspace_open(&f.state, &second).unwrap_err().code,
            "NOT_A_WORKSPACE"
        );
        workspace_create(&f.state, &second, "Second").unwrap();

        workspace_close(&f.state);
        assert_eq!(workspace_current(&f.state).unwrap(), None);

        let names: Vec<String> = workspace_recent(&f.state)
            .into_iter()
            .map(|r| r.name)
            .collect();
        assert_eq!(names, ["Second", "First"]);

        // A recent workspace can be reopened in a later session without the
        // folder picker: a fresh state reads the list from disk.
        let later = AppState::new(f._data.path().to_path_buf());
        let reopened = workspace_open(&later, &info.path).unwrap();
        assert_eq!(reopened.name, "First");
        let names: Vec<String> = workspace_recent(&later)
            .into_iter()
            .map(|r| r.name)
            .collect();
        assert_eq!(names, ["First", "Second"]);
    }

    #[test]
    fn a_deleted_folder_drops_out_of_the_recent_list() {
        let f = fixture();
        let folder = f.picked("gone");
        workspace_create(&f.state, &folder, "Gone").unwrap();
        workspace_close(&f.state);
        std::fs::remove_dir_all(&folder).unwrap();
        assert_eq!(workspace_recent(&f.state), []);
    }

    #[test]
    fn the_tree_commands_work_end_to_end() {
        let f = fixture();
        workspace_create(&f.state, &f.picked("ws"), "Acme").unwrap();

        let collection = collection_create(&f.state, "Petstore").unwrap();
        let folder = folder_create(&f.state, &collection, "Pets").unwrap();
        let list = request_create(&f.state, &collection, "List pets").unwrap();
        let get = request_create(&f.state, &folder, "Get pet").unwrap();

        let tree = workspace_tree(&f.state).unwrap();
        assert_eq!(tree.problems, []);
        let root = &tree.collections[0];
        assert_eq!(
            (root.kind, root.name.as_str()),
            (NodeKind::Collection, "Petstore")
        );
        assert_eq!((root.method.as_deref(), root.id.as_deref()), (None, None));
        let request = &root.children[1];
        assert_eq!(request.kind, NodeKind::Request);
        assert_eq!(request.method.as_deref(), Some("GET"));
        assert_eq!(request.id.as_ref().map(String::len), Some(36));

        let renamed = node_rename(&f.state, &list, "All pets").unwrap();
        assert_eq!(renamed, "collections/petstore/all-pets.yaml");
        let copy = node_duplicate(&f.state, &get).unwrap();
        let moved = node_move(&f.state, &copy, Some(&collection), 0).unwrap();
        assert_eq!(moved, "collections/petstore/get-pet-copy.yaml");
        node_delete(&f.state, &folder).unwrap();

        let tree = workspace_tree(&f.state).unwrap();
        let names: Vec<&str> = tree.collections[0]
            .children
            .iter()
            .map(|n| n.name.as_str())
            .collect();
        assert_eq!(names, ["Get pet copy", "All pets"]);

        collection_create(&f.state, "Second").unwrap();
        node_move(&f.state, "collections/second", None, 0).unwrap();
        let names: Vec<String> = workspace_tree(&f.state)
            .unwrap()
            .collections
            .into_iter()
            .map(|n| n.name)
            .collect();
        assert_eq!(names, ["Second", "Petstore"]);
    }

    #[test]
    fn store_errors_become_stable_codes() {
        let f = fixture();
        workspace_create(&f.state, &f.picked("ws"), "Acme").unwrap();
        let collection = collection_create(&f.state, "Petstore").unwrap();

        let code = |result: Result<String, AppError>| result.unwrap_err().code;
        assert_eq!(code(collection_create(&f.state, "  ")), "EMPTY_NAME");
        assert_eq!(
            code(request_create(&f.state, "../outside", "x")),
            "INVALID_PATH"
        );
        assert_eq!(
            code(request_create(&f.state, "collections/nope", "x")),
            "NOT_FOUND"
        );
        assert_eq!(
            code(node_move(&f.state, &collection, Some(&collection), 0)),
            "INVALID_MOVE"
        );
        assert_eq!(
            code(node_rename(&f.state, "collections", "x")),
            "WRONG_KIND"
        );
    }

    #[test]
    fn opening_repairs_ids_shared_by_copied_files() {
        let f = fixture();
        let folder = f.picked("ws");
        workspace_create(&f.state, &folder, "Acme").unwrap();
        let collection = collection_create(&f.state, "Petstore").unwrap();
        request_create(&f.state, &collection, "A").unwrap();
        let base = std::path::Path::new(&folder).join("collections/petstore");
        std::fs::copy(base.join("a.yaml"), base.join("b.yaml")).unwrap();
        workspace_close(&f.state);

        workspace_open(&f.state, &folder).unwrap();

        let tree = workspace_tree(&f.state).unwrap();
        let ids: Vec<&str> = tree.collections[0]
            .children
            .iter()
            .filter_map(|n| n.id.as_deref())
            .collect();
        assert_eq!(ids.len(), 2);
        assert_ne!(ids[0], ids[1]);
    }
}
