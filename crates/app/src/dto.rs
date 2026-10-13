//! The shapes sent to the frontend.
//!
//! `damnhttp-core` has no serialization or TypeScript concerns, so the
//! types that cross to the webview are defined here and converted from the
//! core ones.

use damnhttp_core::store::{Tree, TreeNode, TreeNodeKind, TreeProblem};
use serde::Serialize;
use specta::Type;

/// The open workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct WorkspaceInfo {
    pub name: String,
    /// The folder on disk, for display.
    pub path: String,
}

/// A workspace opened before, for the start screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize, Type)]
pub struct RecentWorkspace {
    pub name: String,
    pub path: String,
}

/// The sidebar tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct TreeDto {
    pub collections: Vec<TreeNodeDto>,
    /// Files that could not be read; they are not in `collections`.
    pub problems: Vec<TreeProblemDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct TreeNodeDto {
    /// Workspace-relative path; the handle for every tree command.
    pub path: String,
    pub name: String,
    pub kind: NodeKind,
    /// The HTTP method, for requests only.
    pub method: Option<String>,
    /// The stable id, for requests only.
    pub id: Option<String>,
    pub children: Vec<TreeNodeDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    Collection,
    Folder,
    Request,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct TreeProblemDto {
    pub path: String,
    /// Names keys and positions, never values.
    pub message: String,
}

impl From<Tree> for TreeDto {
    fn from(tree: Tree) -> Self {
        Self {
            collections: tree.collections.into_iter().map(Into::into).collect(),
            problems: tree.problems.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<TreeNode> for TreeNodeDto {
    fn from(node: TreeNode) -> Self {
        let (kind, method, id) = match node.kind {
            TreeNodeKind::Collection => (NodeKind::Collection, None, None),
            TreeNodeKind::Folder => (NodeKind::Folder, None, None),
            TreeNodeKind::Request { id, method } => (
                NodeKind::Request,
                Some(method.as_str().to_owned()),
                Some(id.to_string()),
            ),
        };
        Self {
            path: node.path,
            name: node.name,
            kind,
            method,
            id,
            children: node.children.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<TreeProblem> for TreeProblemDto {
    fn from(problem: TreeProblem) -> Self {
        Self {
            path: problem.path,
            message: problem.message,
        }
    }
}
