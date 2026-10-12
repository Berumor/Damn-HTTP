//! What a workspace contains, as shown in the sidebar.

use crate::model::{Method, OrderKey, RequestId};

/// The collections of a workspace with everything inside them, in display
/// order, plus the files that could not be read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tree {
    pub collections: Vec<TreeNode>,
    /// A broken file does not hide the rest of the workspace: it is listed
    /// here and left out of the tree.
    pub problems: Vec<TreeProblem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeNode {
    /// Workspace-relative, with `/` separators. A directory for collections
    /// and folders, a `.yaml` file for requests.
    pub path: String,
    pub name: String,
    pub order: OrderKey,
    pub kind: TreeNodeKind,
    /// Empty for requests.
    pub children: Vec<TreeNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeNodeKind {
    Collection,
    Folder,
    Request { id: RequestId, method: Method },
}

/// A file the tree could not read. `message` names keys and positions,
/// never values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeProblem {
    pub path: String,
    pub message: String,
}

impl Tree {
    /// Every request in the tree as `(path, id)`, depth first.
    pub fn requests(&self) -> Vec<(&str, RequestId)> {
        fn walk<'a>(nodes: &'a [TreeNode], out: &mut Vec<(&'a str, RequestId)>) {
            for node in nodes {
                if let TreeNodeKind::Request { id, .. } = &node.kind {
                    out.push((node.path.as_str(), *id));
                }
                walk(&node.children, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.collections, &mut out);
        out
    }
}
