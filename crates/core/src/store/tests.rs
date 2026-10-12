//! Tests on real temporary directories.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::*;
use crate::model::{Method, PathParam, QueryParam};

const ID_A: &str = "3f2b8c1e-7a4d-4e9b-9c55-0d6a1b2e4f70";

fn workspace() -> (TempDir, WorkspaceStore) {
    let dir = TempDir::new().unwrap();
    let store = WorkspaceStore::create(dir.path(), "Acme APIs").unwrap();
    (dir, store)
}

fn read(store: &WorkspaceStore, path: &str) -> String {
    fs::read_to_string(store.root().join(path)).unwrap()
}

fn write(store: &WorkspaceStore, path: &str, contents: &str) {
    let abs = store.root().join(path);
    fs::create_dir_all(abs.parent().unwrap()).unwrap();
    fs::write(abs, contents).unwrap();
}

fn query(name: &str, value: &str) -> QueryParam {
    QueryParam {
        name: name.to_owned(),
        value: value.to_owned(),
        enabled: true,
        description: String::new(),
    }
}

/// Names of the nodes at one level, in display order.
fn names(nodes: &[TreeNode]) -> Vec<&str> {
    nodes.iter().map(|n| n.name.as_str()).collect()
}

fn paths(nodes: &[TreeNode]) -> Vec<&str> {
    nodes.iter().map(|n| n.path.as_str()).collect()
}

/// A collection with requests `A`, `B`, `C`; returns the collection path.
fn collection_with_abc(store: &WorkspaceStore) -> String {
    let collection = store.create_collection("Petstore").unwrap();
    for name in ["A", "B", "C"] {
        store.create_request(&collection, name).unwrap();
    }
    collection
}

// ----- create and open -----

#[test]
fn create_writes_the_manifest_and_gitignore() {
    let (_dir, store) = workspace();
    assert_eq!(
        read(&store, "damnhttp.yaml"),
        "version: 1\nkind: workspace\nname: Acme APIs\n"
    );
    assert_eq!(read(&store, ".gitignore"), "/.damnhttp/\n");
    assert_eq!(store.workspace().unwrap().name, "Acme APIs");
    assert_eq!(store.tree().unwrap(), Tree::default());
}

#[test]
fn create_keeps_existing_files_and_refuses_an_existing_workspace() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join(".gitignore"), "node_modules/\n").unwrap();
    fs::write(dir.path().join("README.md"), "hello\n").unwrap();

    WorkspaceStore::create(dir.path(), "Acme").unwrap();

    assert_eq!(
        fs::read_to_string(dir.path().join(".gitignore")).unwrap(),
        "node_modules/\n/.damnhttp/\n"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("README.md")).unwrap(),
        "hello\n"
    );
    assert!(matches!(
        WorkspaceStore::create(dir.path(), "Again"),
        Err(StoreError::AlreadyAWorkspace)
    ));
    assert!(matches!(
        WorkspaceStore::create(&dir.path().join("other"), "  "),
        Err(StoreError::EmptyName)
    ));
}

#[test]
fn open_needs_a_manifest_and_restores_the_gitignore_entry() {
    let dir = TempDir::new().unwrap();
    assert!(matches!(
        WorkspaceStore::open(dir.path()),
        Err(StoreError::NotAWorkspace)
    ));
    assert!(matches!(
        WorkspaceStore::open(&dir.path().join("missing")),
        Err(StoreError::NotAWorkspace)
    ));

    WorkspaceStore::create(dir.path(), "Acme").unwrap();
    fs::remove_file(dir.path().join(".gitignore")).unwrap();
    let store = WorkspaceStore::open(dir.path()).unwrap();
    assert_eq!(read(&store, ".gitignore"), "/.damnhttp/\n");
}

#[test]
fn a_workspace_from_a_newer_app_is_refused() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("damnhttp.yaml"),
        "version: 2\nkind: workspace\nname: X\n",
    )
    .unwrap();
    let error = WorkspaceStore::open(dir.path()).unwrap_err();
    assert!(error.to_string().contains("update the app"), "{error}");
}

// ----- creating items -----

#[test]
fn created_items_get_slug_names_and_ascending_order() {
    let (_dir, store) = workspace();
    let collection = store.create_collection("Pet Store").unwrap();
    let folder = store.create_folder(&collection, "Pets & Owners").unwrap();
    let list = store.create_request(&collection, "List pets").unwrap();
    let get = store.create_request(&folder, "Get pet by ID").unwrap();

    assert_eq!(collection, "collections/pet-store");
    assert_eq!(folder, "collections/pet-store/pets-owners");
    assert_eq!(list, "collections/pet-store/list-pets.yaml");
    assert_eq!(get, "collections/pet-store/pets-owners/get-pet-by-id.yaml");

    let tree = store.tree().unwrap();
    assert_eq!(tree.problems, []);
    assert_eq!(names(&tree.collections), ["Pet Store"]);
    let children = &tree.collections[0].children;
    assert_eq!(names(children), ["Pets & Owners", "List pets"]);
    assert_eq!(children[0].order.as_str(), "a0");
    assert_eq!(children[1].order.as_str(), "a1");
    assert_eq!(names(&children[0].children), ["Get pet by ID"]);
    assert!(matches!(
        children[1].kind,
        TreeNodeKind::Request {
            method: Method::Get,
            ..
        }
    ));
}

#[test]
fn name_collisions_get_a_suffix_even_across_kinds_and_case() {
    let (_dir, store) = workspace();
    let collection = store.create_collection("Petstore").unwrap();
    assert_eq!(
        store.create_request(&collection, "Pets").unwrap(),
        "collections/petstore/pets.yaml"
    );
    assert_eq!(
        store.create_request(&collection, "pets!").unwrap(),
        "collections/petstore/pets-2.yaml"
    );
    assert_eq!(
        store.create_folder(&collection, "PETS").unwrap(),
        "collections/petstore/pets-3"
    );
    assert_eq!(
        store.create_request(&collection, "Folder").unwrap(),
        "collections/petstore/folder-item.yaml"
    );
    assert_eq!(
        store.create_collection("petstore").unwrap(),
        "collections/petstore-2"
    );
}

#[test]
fn items_cannot_be_created_in_the_wrong_place() {
    let (_dir, store) = workspace();
    let collection = store.create_collection("Petstore").unwrap();
    let request = store.create_request(&collection, "A").unwrap();

    assert!(matches!(
        store.create_request(COLLECTIONS_DIR, "x"),
        Err(StoreError::WrongKind { .. })
    ));
    assert!(matches!(
        store.create_request(&request, "x"),
        Err(StoreError::WrongKind { .. })
    ));
    assert!(matches!(
        store.create_folder("collections/nope", "x"),
        Err(StoreError::NotFound(_))
    ));
    assert!(matches!(
        store.create_request(&collection, " "),
        Err(StoreError::EmptyName)
    ));
}

// ----- paths -----

#[test]
fn paths_cannot_leave_the_collections_directory() {
    let (dir, store) = workspace();
    let collection = store.create_collection("Petstore").unwrap();
    store.create_request(&collection, "A").unwrap();
    fs::write(dir.path().join("outside.yaml"), "x").unwrap();

    for path in [
        "",
        "damnhttp.yaml",
        "outside.yaml",
        ".damnhttp/values.yaml",
        "collections/../outside.yaml",
        "collections/petstore/../../outside.yaml",
        "collections/petstore/./a.yaml",
        "collections//petstore",
        "/etc/passwd",
        "collections/petstore/.hidden.yaml",
        "collections\\petstore\\a.yaml",
        "collections/C:/x.yaml",
        "collections/petstore/a.yaml/",
    ] {
        assert!(
            matches!(store.read_request(path), Err(StoreError::InvalidPath(_))),
            "{path:?}"
        );
        assert!(
            matches!(store.delete(path), Err(StoreError::InvalidPath(_))),
            "{path:?}"
        );
    }
    assert!(dir.path().join("outside.yaml").exists());
}

#[cfg(unix)]
#[test]
fn symbolic_links_are_not_followed() {
    let (dir, store) = workspace();
    let collection = store.create_collection("Petstore").unwrap();
    let outside = TempDir::new().unwrap();
    fs::write(
        outside.path().join("folder.yaml"),
        "version: 1\nkind: folder\nname: Out\norder: a0\n",
    )
    .unwrap();
    fs::write(outside.path().join("secret.yaml"), "not a request").unwrap();
    let base = dir.path().join("collections/petstore");
    std::os::unix::fs::symlink(outside.path(), base.join("linked")).unwrap();
    std::os::unix::fs::symlink(outside.path().join("secret.yaml"), base.join("link.yaml")).unwrap();

    let tree = store.tree().unwrap();
    assert_eq!(tree.collections[0].children, []);
    assert_eq!(tree.problems, []);
    assert!(
        store
            .read_request(&format!("{collection}/link.yaml"))
            .is_err()
    );
    assert!(
        store
            .create_request(&format!("{collection}/linked"), "x")
            .is_err()
    );
    assert!(store.delete(&format!("{collection}/linked")).is_err());
    assert!(outside.path().join("secret.yaml").exists());
}

// ----- requests and local values -----

/// The maintainer's example: `https://some.service/:country/stations/?id=1234`.
#[test]
fn saving_keeps_literal_values_out_of_the_request_file() {
    let (_dir, store) = workspace();
    let collection = store.create_collection("Stations").unwrap();
    let path = store.create_request(&collection, "Find station").unwrap();

    let mut request = store.read_request(&path).unwrap();
    request.url = "https://some.service/:country/stations/".to_owned();
    request.path_params = vec![PathParam {
        name: "country".to_owned(),
        value: "it".to_owned(),
        description: String::new(),
    }];
    request.query = vec![query("id", "1234"), query("token", "{{apiToken}}")];
    request.skip_tls_verify = true;
    store.save_request(&path, request.clone()).unwrap();

    let file = read(&store, &path);
    assert!(!file.contains("1234"), "{file}");
    assert!(!file.contains("it\n"), "{file}");
    assert!(!file.contains("skip_tls"), "{file}");
    assert!(file.contains("{{apiToken}}"), "{file}");
    let values = read(&store, ".damnhttp/values.yaml");
    assert!(
        values.contains("1234") && values.contains("country: it"),
        "{values}"
    );
    assert!(values.contains("skip_tls_verify: true"), "{values}");

    assert_eq!(store.read_request(&path).unwrap(), request);
}

#[test]
fn a_teammate_without_local_values_sees_empty_fields() {
    let (_dir, store) = workspace();
    let collection = store.create_collection("Stations").unwrap();
    let path = store.create_request(&collection, "Find station").unwrap();
    let mut request = store.read_request(&path).unwrap();
    request.query = vec![query("id", "1234")];
    store.save_request(&path, request).unwrap();

    // What a clone contains: everything except the git-ignored directory.
    fs::remove_dir_all(store.root().join(".damnhttp")).unwrap();

    let seen = store.read_request(&path).unwrap();
    assert_eq!(seen.query, [query("id", "")]);
    assert!(
        !store.root().join(".damnhttp").exists(),
        "reading must not create local state"
    );
}

#[test]
fn clearing_a_value_removes_it_from_the_local_file() {
    let (_dir, store) = workspace();
    let collection = store.create_collection("Stations").unwrap();
    let path = store.create_request(&collection, "Find station").unwrap();
    let mut request = store.read_request(&path).unwrap();
    request.query = vec![query("id", "1234")];
    store.save_request(&path, request.clone()).unwrap();

    request.query = vec![query("id", "")];
    store.save_request(&path, request).unwrap();

    assert_eq!(
        read(&store, ".damnhttp/values.yaml"),
        "version: 1\nkind: values\n"
    );
}

#[test]
fn a_literal_typed_into_the_file_is_moved_out_on_the_next_save() {
    let (_dir, store) = workspace();
    let collection = store.create_collection("Stations").unwrap();
    let path = format!("{collection}/by-hand.yaml");
    write(
        &store,
        &path,
        &format!(
            "id: {ID_A}\nversion: 1\nkind: request\nname: By hand\norder: a0\nmethod: GET\nurl: \"\"\nquery:\n  - name: id\n    value: \"1234\"\n"
        ),
    );

    let request = store.read_request(&path).unwrap();
    assert_eq!(request.query, [query("id", "1234")]);
    assert!(read(&store, ".damnhttp/values.yaml").contains("1234"));

    store.save_request(&path, request).unwrap();
    assert!(!read(&store, &path).contains("1234"));
    assert_eq!(
        store.read_request(&path).unwrap().query,
        [query("id", "1234")]
    );
}

#[test]
fn the_id_of_an_existing_file_cannot_be_replaced() {
    let (_dir, store) = workspace();
    let collection = store.create_collection("Petstore").unwrap();
    let path = store.create_request(&collection, "A").unwrap();
    let mut request = store.read_request(&path).unwrap();
    request.id = RequestId::generate();
    assert!(matches!(
        store.save_request(&path, request),
        Err(StoreError::IdMismatch(_))
    ));
}

#[test]
fn saving_unchanged_content_does_not_touch_the_file() {
    let (_dir, store) = workspace();
    let collection = store.create_collection("Petstore").unwrap();
    let path = store.create_request(&collection, "A").unwrap();
    let abs = store.root().join(&path);
    let before = fs::metadata(&abs).unwrap().modified().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));

    let request = store.read_request(&path).unwrap();
    store.save_request(&path, request).unwrap();

    assert_eq!(fs::metadata(&abs).unwrap().modified().unwrap(), before);
    let leftovers: Vec<_> = fs::read_dir(abs.parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.ends_with(".tmp"))
        .collect();
    assert_eq!(leftovers, Vec::<String>::new());
}

#[test]
fn collection_and_folder_settings_round_trip() {
    let (_dir, store) = workspace();
    let collection_path = store.create_collection("Petstore").unwrap();
    let folder_path = store.create_folder(&collection_path, "Pets").unwrap();

    let mut collection = store.read_collection(&collection_path).unwrap();
    collection.description = "All about pets".to_owned();
    collection.auth = Auth::Bearer {
        token: "{{apiToken}}".to_owned(),
    };
    store
        .save_collection(&collection_path, &collection)
        .unwrap();
    assert_eq!(store.read_collection(&collection_path).unwrap(), collection);

    let mut folder = store.read_folder(&folder_path).unwrap();
    folder.auth = Auth::None;
    store.save_folder(&folder_path, &folder).unwrap();
    assert_eq!(store.read_folder(&folder_path).unwrap(), folder);

    assert!(matches!(
        store.read_folder(&collection_path),
        Err(StoreError::WrongKind { .. })
    ));
    assert!(matches!(
        store.read_collection(&folder_path),
        Err(StoreError::WrongKind { .. })
    ));
}

// ----- rename -----

#[test]
fn renaming_a_request_renames_the_file_and_keeps_id_and_local_values() {
    let (_dir, store) = workspace();
    let collection = store.create_collection("Petstore").unwrap();
    let path = store.create_request(&collection, "Old name").unwrap();
    let mut request = store.read_request(&path).unwrap();
    request.query = vec![query("id", "1234")];
    store.save_request(&path, request.clone()).unwrap();

    let new_path = store.rename(&path, "  New name ").unwrap();

    assert_eq!(new_path, "collections/petstore/new-name.yaml");
    assert!(!store.root().join(&path).exists());
    let renamed = store.read_request(&new_path).unwrap();
    assert_eq!(renamed.name, "New name");
    assert_eq!(renamed.id, request.id);
    assert_eq!(renamed.query, [query("id", "1234")]);
}

#[test]
fn renaming_a_folder_moves_its_contents_and_avoids_collisions() {
    let (_dir, store) = workspace();
    let collection = store.create_collection("Petstore").unwrap();
    let folder = store.create_folder(&collection, "Pets").unwrap();
    store.create_folder(&collection, "Owners").unwrap();
    store.create_request(&folder, "Get pet").unwrap();

    let renamed = store.rename(&folder, "Owners").unwrap();
    assert_eq!(renamed, "collections/petstore/owners-2");
    assert_eq!(store.read_folder(&renamed).unwrap().name, "Owners");
    assert!(
        store
            .read_request("collections/petstore/owners-2/get-pet.yaml")
            .is_ok()
    );

    // Same slug, different display name: the path stays.
    assert_eq!(store.rename(&renamed, "OWNERS 2").unwrap(), renamed);
    assert_eq!(store.read_folder(&renamed).unwrap().name, "OWNERS 2");

    let collection = store.rename(&collection, "Pet shop").unwrap();
    assert_eq!(collection, "collections/pet-shop");
    assert_eq!(store.tree().unwrap().collections[0].name, "Pet shop");
}

// ----- duplicate -----

#[test]
fn duplicating_a_request_gives_a_new_id_and_copies_local_values() {
    let (_dir, store) = workspace();
    let collection = collection_with_abc(&store);
    let a = format!("{collection}/a.yaml");
    let mut request = store.read_request(&a).unwrap();
    request.query = vec![query("id", "1234")];
    store.save_request(&a, request.clone()).unwrap();

    let copy_path = store.duplicate(&a).unwrap();

    assert_eq!(copy_path, "collections/petstore/a-copy.yaml");
    let copy = store.read_request(&copy_path).unwrap();
    assert_eq!(copy.name, "A copy");
    assert_ne!(copy.id, request.id);
    assert_eq!(copy.query, [query("id", "1234")]);
    let tree = store.tree().unwrap();
    assert_eq!(
        names(&tree.collections[0].children),
        ["A", "A copy", "B", "C"]
    );
}

#[test]
fn duplicating_a_folder_copies_everything_with_new_ids() {
    let (_dir, store) = workspace();
    let collection = store.create_collection("Petstore").unwrap();
    let folder = store.create_folder(&collection, "Pets").unwrap();
    let inner = store.create_folder(&folder, "Inner").unwrap();
    store.create_request(&folder, "One").unwrap();
    store.create_request(&inner, "Two").unwrap();

    let copy = store.duplicate(&folder).unwrap();

    assert_eq!(copy, "collections/petstore/pets-copy");
    let tree = store.tree().unwrap();
    let children = &tree.collections[0].children;
    assert_eq!(names(children), ["Pets", "Pets copy"]);
    assert_eq!(names(&children[1].children), ["Inner", "One"]);
    assert_eq!(names(&children[1].children[0].children), ["Two"]);
    let ids: HashSet<RequestId> = tree.requests().into_iter().map(|(_, id)| id).collect();
    assert_eq!(ids.len(), 4);

    let collection_copy = store.duplicate(&collection).unwrap();
    assert_eq!(collection_copy, "collections/petstore-copy");
    let tree = store.tree().unwrap();
    assert_eq!(names(&tree.collections), ["Petstore", "Petstore copy"]);
    assert_eq!(tree.requests().len(), 8);
}

// ----- move -----

#[test]
fn reordering_rewrites_only_the_moved_file() {
    let (_dir, store) = workspace();
    let collection = collection_with_abc(&store);
    let before_a = read(&store, &format!("{collection}/a.yaml"));
    let before_b = read(&store, &format!("{collection}/b.yaml"));

    let moved = store
        .move_to(&format!("{collection}/c.yaml"), &collection, 0)
        .unwrap();

    assert_eq!(moved, format!("{collection}/c.yaml"));
    let tree = store.tree().unwrap();
    assert_eq!(names(&tree.collections[0].children), ["C", "A", "B"]);
    assert_eq!(read(&store, &format!("{collection}/a.yaml")), before_a);
    assert_eq!(read(&store, &format!("{collection}/b.yaml")), before_b);

    store
        .move_to(&format!("{collection}/c.yaml"), &collection, 1)
        .unwrap();
    assert_eq!(
        names(&store.tree().unwrap().collections[0].children),
        ["A", "C", "B"]
    );
    store
        .move_to(&format!("{collection}/a.yaml"), &collection, 99)
        .unwrap();
    assert_eq!(
        names(&store.tree().unwrap().collections[0].children),
        ["C", "B", "A"]
    );
}

#[test]
fn moving_into_another_folder_keeps_id_and_avoids_collisions() {
    let (_dir, store) = workspace();
    let collection = collection_with_abc(&store);
    let folder = store.create_folder(&collection, "Pets").unwrap();
    store.create_request(&folder, "A").unwrap();
    let a = format!("{collection}/a.yaml");
    let id = store.read_request(&a).unwrap().id;

    let moved = store.move_to(&a, &folder, 0).unwrap();

    assert_eq!(moved, "collections/petstore/pets/a-2.yaml");
    assert_eq!(store.read_request(&moved).unwrap().id, id);
    let tree = store.tree().unwrap();
    assert_eq!(names(&tree.collections[0].children), ["B", "C", "Pets"]);
    assert_eq!(
        paths(&tree.collections[0].children[2].children),
        [
            "collections/petstore/pets/a-2.yaml",
            "collections/petstore/pets/a.yaml"
        ]
    );
}

#[test]
fn collections_can_be_reordered() {
    let (_dir, store) = workspace();
    for name in ["One", "Two", "Three"] {
        store.create_collection(name).unwrap();
    }
    store
        .move_to("collections/three", COLLECTIONS_DIR, 0)
        .unwrap();
    assert_eq!(
        names(&store.tree().unwrap().collections),
        ["Three", "One", "Two"]
    );
}

#[test]
fn invalid_moves_are_refused() {
    let (_dir, store) = workspace();
    let collection = collection_with_abc(&store);
    let folder = store.create_folder(&collection, "Pets").unwrap();
    let inner = store.create_folder(&folder, "Inner").unwrap();
    let a = format!("{collection}/a.yaml");

    assert!(matches!(
        store.move_to(&folder, &folder, 0),
        Err(StoreError::InvalidMove(_))
    ));
    assert!(matches!(
        store.move_to(&folder, &inner, 0),
        Err(StoreError::InvalidMove(_))
    ));
    assert!(matches!(
        store.move_to(&a, COLLECTIONS_DIR, 0),
        Err(StoreError::InvalidMove(_))
    ));
    assert!(matches!(
        store.move_to(&collection, &folder, 0),
        Err(StoreError::InvalidMove(_))
    ));
    assert!(matches!(
        store.move_to(&a, &format!("{collection}/b.yaml"), 0),
        Err(StoreError::WrongKind { .. })
    ));
    assert_eq!(
        names(&store.tree().unwrap().collections[0].children),
        ["A", "B", "C", "Pets"]
    );
}

#[test]
fn hand_written_or_equal_order_keys_are_renormalized_when_needed() {
    let (_dir, store) = workspace();
    let collection = collection_with_abc(&store);
    // Two people gave their new request the same key; ties sort by file name.
    for name in ["a", "b", "c"] {
        let path = format!("{collection}/{name}.yaml");
        let text = read(&store, &path);
        let order = if name == "c" { "order: 7" } else { "order: a0" };
        let text: String = text
            .lines()
            .map(|l| if l.starts_with("order: ") { order } else { l })
            .map(|l| format!("{l}\n"))
            .collect();
        write(&store, &path, &text);
    }
    assert_eq!(
        names(&store.tree().unwrap().collections[0].children),
        ["C", "A", "B"]
    );

    // Between the two equal keys there is no room: the group is rewritten.
    let d = store.create_request(&collection, "D").unwrap();
    store.move_to(&d, &collection, 2).unwrap();

    let tree = store.tree().unwrap();
    let children = &tree.collections[0].children;
    assert_eq!(names(children), ["C", "A", "D", "B"]);
    let orders: Vec<&str> = children.iter().map(|n| n.order.as_str()).collect();
    assert_eq!(orders, ["a0", "a1", "a2", "a3"]);
}

#[test]
fn order_keys_stay_short_under_repeated_inserts_at_one_place() {
    let (_dir, store) = workspace();
    let collection = collection_with_abc(&store);
    for i in 0..40 {
        let path = store.create_request(&collection, &format!("N{i}")).unwrap();
        store.move_to(&path, &collection, 1).unwrap();
    }
    let tree = store.tree().unwrap();
    let children = &tree.collections[0].children;
    assert_eq!(children.len(), 43);
    assert_eq!(children[0].name, "A");
    assert_eq!(children[1].name, "N39");
    assert!(
        children
            .iter()
            .all(|n| n.order.as_str().len() <= MAX_ORDER_KEY_LENGTH)
    );
    assert!(
        children
            .windows(2)
            .all(|pair| pair[0].order < pair[1].order)
    );
}

// ----- delete -----

#[test]
fn deleting_removes_files_but_keeps_local_values_until_cleaned() {
    let (_dir, store) = workspace();
    let collection = collection_with_abc(&store);
    let folder = store.create_folder(&collection, "Pets").unwrap();
    let inner = store.create_request(&folder, "Inner").unwrap();
    for path in [format!("{collection}/a.yaml"), inner.clone()] {
        let mut request = store.read_request(&path).unwrap();
        request.query = vec![query("id", "1234")];
        store.save_request(&path, request).unwrap();
    }

    store.delete(&folder).unwrap();
    store.delete(&format!("{collection}/b.yaml")).unwrap();

    assert_eq!(
        names(&store.tree().unwrap().collections[0].children),
        ["A", "C"]
    );
    assert!(matches!(
        store.delete(&folder),
        Err(StoreError::NotFound(_))
    ));
    assert_eq!(store.clean_local_values().unwrap(), 1);
    assert_eq!(store.clean_local_values().unwrap(), 0);
    assert_eq!(
        store
            .read_request(&format!("{collection}/a.yaml"))
            .unwrap()
            .query,
        [query("id", "1234")]
    );
}

// ----- problems and repairs -----

#[test]
fn a_broken_file_is_reported_and_the_rest_still_loads() {
    let (_dir, store) = workspace();
    let collection = collection_with_abc(&store);
    write(
        &store,
        &format!("{collection}/broken.yaml"),
        "kind: request\nname: [unclosed\n",
    );
    write(&store, &format!("{collection}/notes.txt"), "not ours");
    fs::create_dir(store.root().join(&collection).join("no-marker")).unwrap();

    let tree = store.tree().unwrap();

    assert_eq!(names(&tree.collections[0].children), ["A", "B", "C"]);
    assert_eq!(tree.problems.len(), 1);
    assert_eq!(tree.problems[0].path, format!("{collection}/broken.yaml"));
    assert!(
        tree.problems[0].message.starts_with("line "),
        "{}",
        tree.problems[0].message
    );
    assert!(matches!(
        store.clean_local_values(),
        Err(StoreError::UnreadableFiles(1))
    ));
    assert!(matches!(
        store.duplicate(&collection),
        Err(StoreError::UnreadableFiles(1))
    ));
    assert!(!store.root().join("collections/petstore-copy").exists());
}

#[test]
fn a_file_copied_by_hand_gets_a_new_id_and_keeps_the_values() {
    let (_dir, store) = workspace();
    let collection = collection_with_abc(&store);
    let b = format!("{collection}/b.yaml");
    let mut request = store.read_request(&b).unwrap();
    request.query = vec![query("id", "1234")];
    store.save_request(&b, request.clone()).unwrap();
    let copy = format!("{collection}/zz-copy.yaml");
    fs::copy(store.root().join(&b), store.root().join(&copy)).unwrap();

    assert_eq!(store.repair_duplicate_ids().unwrap(), [copy.as_str()]);

    assert_eq!(store.read_request(&b).unwrap().id, request.id);
    let repaired = store.read_request(&copy).unwrap();
    assert_ne!(repaired.id, request.id);
    assert_eq!(repaired.query, [query("id", "1234")]);
    assert_eq!(store.repair_duplicate_ids().unwrap(), Vec::<String>::new());
}

#[test]
fn unreadable_local_values_stop_a_save_before_anything_is_written() {
    let (_dir, store) = workspace();
    let collection = collection_with_abc(&store);
    let a = format!("{collection}/a.yaml");
    let before = read(&store, &a);
    write(
        &store,
        ".damnhttp/values.yaml",
        "version: 1\nkind: values\nrequests: [broken\n",
    );

    let mut request = Request::new("A", OrderKey::first());
    request.query = vec![query("id", "1234")];
    let error = store.store_request(&a, request).unwrap_err();

    assert!(matches!(error, StoreError::Format { .. }), "{error}");
    assert_eq!(read(&store, &a), before);
    assert!(!error.to_string().contains("1234"));
}

#[test]
fn every_written_file_is_inside_the_workspace_and_ends_in_yaml() {
    fn walk(dir: &Path, out: &mut Vec<String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                walk(&entry.path(), out);
            } else {
                out.push(entry.file_name().into_string().unwrap());
            }
        }
    }
    let (dir, store) = workspace();
    let collection = store.create_collection("../../Escape attempt").unwrap();
    let folder = store.create_folder(&collection, "..").unwrap();
    store.create_request(&folder, "a/b\\c:d").unwrap();
    store.create_request(&folder, "日本語").unwrap();

    let mut files = Vec::new();
    walk(dir.path(), &mut files);
    files.sort();
    assert_eq!(
        files,
        [
            ".gitignore",
            "a-b-c-d.yaml",
            "collection.yaml",
            "damnhttp.yaml",
            "folder.yaml",
            "untitled.yaml"
        ]
    );
    assert_eq!(folder, "collections/escape-attempt/untitled");
}
