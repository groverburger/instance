//! Non-UI project logic: resource creation/deletion, resource-tree manipulation,
//! loading and saving (specs/ide.md §2, §3.3, §3.10).
//!
//! Everything here is pure data manipulation on [`gm_format::Project`] so it can be
//! unit-tested without a window.

use std::path::Path;

use gm_format::{FormatVersion, NodeStatus, Project, ResourceKind, TreeNode};

/// Index path into `Project::resource_tree` (first element = root index).
pub type TreePath = Vec<usize>;

/// The nine list resource kinds in tree order, with display names and default name prefixes.
pub const LIST_KINDS: [(ResourceKind, &str, &str); 9] = [
    (ResourceKind::Sprite, "Sprite", "sprite"),
    (ResourceKind::Sound, "Sound", "sound"),
    (ResourceKind::Background, "Background", "background"),
    (ResourceKind::Path, "Path", "path"),
    (ResourceKind::Script, "Script", "script"),
    (ResourceKind::Font, "Font", "font"),
    (ResourceKind::Timeline, "Time Line", "timeline"),
    (ResourceKind::Object, "Object", "object"),
    (ResourceKind::Room, "Room", "room"),
];

/// Human readable singular name of a list kind.
pub fn kind_label(kind: ResourceKind) -> &'static str {
    LIST_KINDS
        .iter()
        .find(|k| k.0 == kind)
        .map_or("Resource", |k| k.1)
}

/// Default-name prefix of a list kind.
pub fn kind_prefix(kind: ResourceKind) -> &'static str {
    LIST_KINDS
        .iter()
        .find(|k| k.0 == kind)
        .map_or("resource", |k| k.2)
}

/// Whether `kind` is one of the nine list kinds.
pub fn is_list_kind(kind: ResourceKind) -> bool {
    LIST_KINDS.iter().any(|k| k.0 == kind)
}

/// Run `$body` with `$list` bound to the `Vec<Option<T>>` of `$kind`.
/// Evaluates `$other` for non-list kinds.
macro_rules! with_list {
    ($project:expr, $kind:expr, |$list:ident| $body:expr, $other:expr) => {
        match $kind {
            ResourceKind::Sprite => {
                let $list = &$project.sprites;
                $body
            }
            ResourceKind::Sound => {
                let $list = &$project.sounds;
                $body
            }
            ResourceKind::Background => {
                let $list = &$project.backgrounds;
                $body
            }
            ResourceKind::Path => {
                let $list = &$project.paths;
                $body
            }
            ResourceKind::Script => {
                let $list = &$project.scripts;
                $body
            }
            ResourceKind::Font => {
                let $list = &$project.fonts;
                $body
            }
            ResourceKind::Timeline => {
                let $list = &$project.timelines;
                $body
            }
            ResourceKind::Object => {
                let $list = &$project.objects;
                $body
            }
            ResourceKind::Room => {
                let $list = &$project.rooms;
                $body
            }
            _ => $other,
        }
    };
}

macro_rules! with_list_mut {
    ($project:expr, $kind:expr, |$list:ident| $body:expr, $other:expr) => {
        match $kind {
            ResourceKind::Sprite => {
                let $list = &mut $project.sprites;
                $body
            }
            ResourceKind::Sound => {
                let $list = &mut $project.sounds;
                $body
            }
            ResourceKind::Background => {
                let $list = &mut $project.backgrounds;
                $body
            }
            ResourceKind::Path => {
                let $list = &mut $project.paths;
                $body
            }
            ResourceKind::Script => {
                let $list = &mut $project.scripts;
                $body
            }
            ResourceKind::Font => {
                let $list = &mut $project.fonts;
                $body
            }
            ResourceKind::Timeline => {
                let $list = &mut $project.timelines;
                $body
            }
            ResourceKind::Object => {
                let $list = &mut $project.objects;
                $body
            }
            ResourceKind::Room => {
                let $list = &mut $project.rooms;
                $body
            }
            _ => $other,
        }
    };
}

/// Trait giving uniform access to the name of every list resource type.
pub trait Named: Clone {
    fn name(&self) -> &str;
    fn set_name(&mut self, n: String);
    fn set_last_changed(&mut self, t: f64);
    fn new_named(n: String) -> Self;
}

macro_rules! impl_named {
    ($($t:ty),*) => {$(
        impl Named for $t {
            fn name(&self) -> &str { &self.name }
            fn set_name(&mut self, n: String) { self.name = n; }
            fn set_last_changed(&mut self, t: f64) { self.last_changed = t; }
            fn new_named(n: String) -> Self { Self { name: n, ..Default::default() } }
        }
    )*};
}
impl_named!(
    gm_format::Sprite,
    gm_format::Sound,
    gm_format::Background,
    gm_format::Path,
    gm_format::Script,
    gm_format::Font,
    gm_format::Timeline,
    gm_format::Object,
    gm_format::Room
);

/// Number of slots of a list kind.
pub fn slot_count(p: &Project, kind: ResourceKind) -> usize {
    with_list!(p, kind, |l| l.len(), 0)
}

/// Name of resource `idx` of `kind`, if it exists.
pub fn res_name(p: &Project, kind: ResourceKind, idx: usize) -> Option<&str> {
    with_list!(
        p,
        kind,
        |l| l.get(idx).and_then(|x| x.as_ref()).map(|x| x.name()),
        None
    )
}

/// Whether resource `idx` of `kind` exists.
#[cfg_attr(not(test), allow(dead_code))]
pub fn res_exists(p: &Project, kind: ResourceKind, idx: usize) -> bool {
    res_name(p, kind, idx).is_some()
}

/// Rename a resource and its tree leaf.
pub fn rename_resource(p: &mut Project, kind: ResourceKind, idx: usize, name: &str) {
    with_list_mut!(
        p,
        kind,
        |l| if let Some(Some(r)) = l.get_mut(idx) {
            r.set_name(name.to_string());
        },
        ()
    );
    sync_tree_name(p, kind, idx);
}

/// Copy the resource's name into its tree leaf (after a commit).
pub fn sync_tree_name(p: &mut Project, kind: ResourceKind, idx: usize) {
    let Some(name) = res_name(p, kind, idx).map(str::to_string) else {
        return;
    };
    if let Some(path) = find_leaf(p, kind, idx) {
        if let Some(n) = node_mut(&mut p.resource_tree, &path) {
            n.name = name;
        }
    }
}

/// Current time as a Delphi `TDateTime` (UTC; see OPEN_QUESTIONS [IMPL-IDE]).
pub fn now_timestamp() -> f64 {
    let secs = crate::platform::unix_secs();
    secs / 86400.0 + 25569.0
}

/// Whether any resource of any list kind is called `name`.
pub fn name_in_use(p: &Project, name: &str) -> bool {
    LIST_KINDS.iter().any(|(k, _, _)| {
        with_list!(
            p,
            *k,
            |l| l.iter().flatten().any(|r| r.name() == name),
            false
        )
    })
}

/// Default name for a new resource of `kind` with id `idx` (spec §2: N = new id),
/// bumped until unique.
pub fn default_name(p: &Project, kind: ResourceKind, idx: usize) -> String {
    let prefix = kind_prefix(kind);
    let mut n = idx;
    loop {
        let name = format!("{prefix}{n}");
        if !name_in_use(p, &name) {
            return name;
        }
        n += 1;
    }
}

// ------------------------------------------------------------------- tree helpers

/// Node at `path`.
pub fn node<'a>(roots: &'a [TreeNode], path: &[usize]) -> Option<&'a TreeNode> {
    let (first, rest) = path.split_first()?;
    let mut n = roots.get(*first)?;
    for &i in rest {
        n = n.children.get(i)?;
    }
    Some(n)
}

/// Mutable node at `path`.
pub fn node_mut<'a>(roots: &'a mut [TreeNode], path: &[usize]) -> Option<&'a mut TreeNode> {
    let (first, rest) = path.split_first()?;
    let mut n = roots.get_mut(*first)?;
    for &i in rest {
        n = n.children.get_mut(i)?;
    }
    Some(n)
}

/// Index of the root node of `kind`.
pub fn root_index(p: &Project, kind: ResourceKind) -> Option<usize> {
    p.resource_tree.iter().position(|n| n.kind == kind)
}

/// Path of the leaf for resource `idx` of `kind`.
pub fn find_leaf(p: &Project, kind: ResourceKind, idx: usize) -> Option<TreePath> {
    fn go(n: &TreeNode, kind: ResourceKind, idx: usize, path: &mut TreePath) -> bool {
        if n.status == NodeStatus::Leaf && n.kind == kind && n.index == idx as i32 {
            return true;
        }
        for (i, c) in n.children.iter().enumerate() {
            path.push(i);
            if go(c, kind, idx, path) {
                return true;
            }
            path.pop();
        }
        false
    }
    let r = root_index(p, kind)?;
    let mut path = vec![r];
    if go(&p.resource_tree[r], kind, idx, &mut path) && path.len() > 1 {
        Some(path)
    } else {
        None
    }
}

/// Resources of `kind` in tree order as `(index, name, depth)`; `depth` counts group
/// nesting (0 = directly under the root). Groups are reported with index `-1`.
pub fn tree_order(p: &Project, kind: ResourceKind) -> Vec<(i32, String, usize)> {
    let mut out = Vec::new();
    fn go(n: &TreeNode, depth: usize, out: &mut Vec<(i32, String, usize)>) {
        for c in &n.children {
            match c.status {
                NodeStatus::Leaf => out.push((c.index, c.name.clone(), depth)),
                _ => {
                    out.push((-1, c.name.clone(), depth));
                    go(c, depth + 1, out);
                }
            }
        }
    }
    if let Some(r) = root_index(p, kind) {
        go(&p.resource_tree[r], 0, &mut out);
    }
    out
}

/// Recompute `room_order` from the room tree (spec §2: first room = start room).
pub fn rebuild_room_order(p: &mut Project) {
    p.room_order = tree_order(p, ResourceKind::Room)
        .into_iter()
        .filter(|x| x.0 >= 0)
        .map(|x| x.0)
        .collect();
}

/// Insert `n` as child `pos` (clamped) of the node at `parent`.
fn insert_child(p: &mut Project, parent: &[usize], pos: Option<usize>, n: TreeNode) -> TreePath {
    let parent_node = node_mut(&mut p.resource_tree, parent).expect("valid parent path");
    let pos = pos
        .unwrap_or(parent_node.children.len())
        .min(parent_node.children.len());
    parent_node.children.insert(pos, n);
    let mut path = parent.to_vec();
    path.push(pos);
    path
}

/// Where a new resource goes in the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placement {
    /// Appended at the end of the kind's root.
    End,
    /// Appended inside the group at this path.
    Into(TreePath),
    /// Inserted before the node at this path (Edit > Insert).
    Before(TreePath),
    /// Inserted after the node at this path (Duplicate).
    After(TreePath),
}

fn resolve_placement(p: &Project, kind: ResourceKind, at: &Placement) -> (TreePath, Option<usize>) {
    let root = vec![root_index(p, kind).expect("standard roots present")];
    let same_kind = |path: &[usize]| node(&p.resource_tree, path).is_some_and(|n| n.kind == kind);
    match at {
        Placement::Into(g) if same_kind(g) => (g.clone(), None),
        Placement::Before(t) if t.len() > 1 && same_kind(t) => {
            (t[..t.len() - 1].to_vec(), Some(t[t.len() - 1]))
        }
        Placement::After(t) if t.len() > 1 && same_kind(t) => {
            (t[..t.len() - 1].to_vec(), Some(t[t.len() - 1] + 1))
        }
        _ => (root, None),
    }
}

/// Create a new resource of a list kind with default values and a default name.
/// Returns its index and tree path.
pub fn create_resource(p: &mut Project, kind: ResourceKind, at: &Placement) -> (usize, TreePath) {
    let idx = slot_count(p, kind);
    let name = default_name(p, kind, idx);
    with_list_mut!(p, kind, |l| l.push(Some(Named::new_named(name.clone()))), {
        panic!("not a list kind")
    });
    stamp(p, kind, idx);
    let (parent, pos) = resolve_placement(p, kind, at);
    let path = insert_child(p, &parent, pos, TreeNode::leaf(kind, idx as i32, name));
    if kind == ResourceKind::Room {
        rebuild_room_order(p);
    }
    (idx, path)
}

/// Set `last_changed` of a resource to now.
pub fn stamp(p: &mut Project, kind: ResourceKind, idx: usize) {
    let t = now_timestamp();
    with_list_mut!(
        p,
        kind,
        |l| if let Some(Some(r)) = l.get_mut(idx) {
            r.set_last_changed(t);
        },
        ()
    );
}

/// Duplicate resource `idx`; the copy gets a new name and is placed after the original.
pub fn duplicate_resource(
    p: &mut Project,
    kind: ResourceKind,
    idx: usize,
) -> Option<(usize, TreePath)> {
    let new_idx = slot_count(p, kind);
    let base = res_name(p, kind, idx)?.to_string();
    let mut name = format!("{base}_copy");
    let mut k = 1;
    while name_in_use(p, &name) {
        k += 1;
        name = format!("{base}_copy{k}");
    }
    with_list_mut!(
        p,
        kind,
        |l| {
            let mut c = l[idx].clone().expect("exists");
            c.set_name(name.clone());
            l.push(Some(c));
        },
        return None
    );
    // Rooms: instance/tile ids must stay unique (spec §15.4).
    if kind == ResourceKind::Room {
        let mut next_inst = p.last_instance_id;
        let mut next_tile = p.last_tile_id;
        if let Some(Some(r)) = p.rooms.get_mut(new_idx) {
            for i in &mut r.instances {
                next_inst += 1;
                i.id = next_inst;
            }
            for t in &mut r.tiles {
                next_tile += 1;
                t.id = next_tile;
            }
        }
        p.last_instance_id = next_inst;
        p.last_tile_id = next_tile;
    }
    stamp(p, kind, new_idx);
    let at = match find_leaf(p, kind, idx) {
        Some(path) => Placement::After(path),
        None => Placement::End,
    };
    let (parent, pos) = resolve_placement(p, kind, &at);
    let path = insert_child(p, &parent, pos, TreeNode::leaf(kind, new_idx as i32, name));
    if kind == ResourceKind::Room {
        rebuild_room_order(p);
    }
    Some((new_idx, path))
}

/// Add a group named `name` inside the node at `parent` (root or group).
pub fn add_group(p: &mut Project, parent: &[usize], name: &str) -> Option<TreePath> {
    let n = node(&p.resource_tree, parent)?;
    if n.status == NodeStatus::Leaf {
        return None;
    }
    let kind = n.kind;
    Some(insert_child(
        p,
        parent,
        None,
        TreeNode::group(kind, name, Vec::new()),
    ))
}

/// Delete the node at `path` (a leaf or a group with all its contents).
/// Returns the resources that were deleted. Roots cannot be deleted.
pub fn delete_node(p: &mut Project, path: &[usize]) -> Vec<(ResourceKind, usize)> {
    if path.len() < 2 {
        return Vec::new();
    }
    let parent = &path[..path.len() - 1];
    let Some(par) = node_mut(&mut p.resource_tree, parent) else {
        return Vec::new();
    };
    let i = path[path.len() - 1];
    if i >= par.children.len() {
        return Vec::new();
    }
    let removed = par.children.remove(i);
    let mut deleted = Vec::new();
    removed.walk(&mut |n, _| {
        if n.status == NodeStatus::Leaf && n.index >= 0 {
            deleted.push((n.kind, n.index as usize));
        }
    });
    for &(kind, idx) in &deleted {
        with_list_mut!(
            p,
            kind,
            |l| if let Some(s) = l.get_mut(idx) {
                *s = None;
            },
            ()
        );
    }
    if deleted.iter().any(|d| d.0 == ResourceKind::Room) {
        rebuild_room_order(p);
    }
    deleted
}

/// Whether `a` is `b` or an ancestor of `b`.
fn is_prefix(a: &[usize], b: &[usize]) -> bool {
    a.len() <= b.len() && b[..a.len()] == *a
}

/// Move the node at `from` to become child `pos` of the node `to_parent`.
/// Only allowed within one kind and not into the node's own subtree.
/// Returns the new path.
pub fn move_node(
    p: &mut Project,
    from: &[usize],
    to_parent: &[usize],
    pos: usize,
) -> Option<TreePath> {
    if from.len() < 2 || is_prefix(from, to_parent) {
        return None;
    }
    let src = node(&p.resource_tree, from)?;
    let dst = node(&p.resource_tree, to_parent)?;
    if src.kind != dst.kind || dst.status == NodeStatus::Leaf {
        return None;
    }
    let kind = src.kind;
    let from_parent = &from[..from.len() - 1];
    let from_i = from[from.len() - 1];
    // Adjust target path/position for the removal.
    let mut to_parent = to_parent.to_vec();
    let mut pos = pos;
    if to_parent.len() > from_parent.len()
        && to_parent[..from_parent.len()] == *from_parent
        && to_parent[from_parent.len()] > from_i
    {
        to_parent[from_parent.len()] -= 1;
    }
    if to_parent == from_parent && pos > from_i {
        pos -= 1;
    }
    let n = node_mut(&mut p.resource_tree, from_parent)?
        .children
        .remove(from_i);
    let path = insert_child(p, &to_parent, Some(pos), n);
    if kind == ResourceKind::Room {
        rebuild_room_order(p);
    }
    Some(path)
}

// ------------------------------------------------------------------- load / save

/// Read a project file (any supported version).
pub fn load_project(path: &Path) -> Result<Project, String> {
    gm_format::read_project_file(path)
        .map(|(p, _)| p)
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Serialize a project in the native `.gm81` (v810) format.
pub fn project_bytes(p: &Project) -> Result<Vec<u8>, String> {
    gm_format::write_project(p, FormatVersion::V810).map_err(|e| e.to_string())
}

/// Save a project as `.gm81`, writing to a temporary file first.
pub fn save_project(p: &Project, path: &Path) -> Result<(), String> {
    let bytes = project_bytes(p)?;
    let tmp = path.with_extension("gm81~");
    std::fs::write(&tmp, &bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

/// A new empty project as created by File > New.
pub fn new_project() -> Project {
    let mut p = Project::default();
    // Random-ish game id and guid (the IDE's "unique game id").
    let seed = crate::platform::unix_nanos().max(1);
    let mut x = seed ^ 0x9E37_79B9_7F4A_7C15;
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    p.game_id = (next() % 100_000_000) as i32;
    for b in p.guid.iter_mut() {
        *b = next() as u8;
    }
    let now = now_timestamp();
    p.settings.last_changed = now;
    p.game_info.last_changed = now;
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_delete() {
        let mut p = new_project();
        let (i, path) = create_resource(&mut p, ResourceKind::Sprite, &Placement::End);
        assert_eq!(i, 0);
        assert_eq!(res_name(&p, ResourceKind::Sprite, 0), Some("sprite0"));
        assert_eq!(find_leaf(&p, ResourceKind::Sprite, 0), Some(path.clone()));
        let (j, _) = create_resource(
            &mut p,
            ResourceKind::Sprite,
            &Placement::Before(path.clone()),
        );
        assert_eq!(j, 1);
        // inserted before: index 1 is first child now
        assert_eq!(find_leaf(&p, ResourceKind::Sprite, 1).unwrap(), path);
        let leaf = find_leaf(&p, ResourceKind::Sprite, 0).unwrap();
        let del = delete_node(&mut p, &leaf);
        assert_eq!(del, vec![(ResourceKind::Sprite, 0)]);
        assert!(p.sprites[0].is_none());
        // new resource never reuses a deleted slot
        let (k, _) = create_resource(&mut p, ResourceKind::Sprite, &Placement::End);
        assert_eq!(k, 2);
    }

    #[test]
    fn rooms_order_follows_tree() {
        let mut p = new_project();
        create_resource(&mut p, ResourceKind::Room, &Placement::End);
        create_resource(&mut p, ResourceKind::Room, &Placement::End);
        assert_eq!(p.room_order, vec![0, 1]);
        let r = root_index(&p, ResourceKind::Room).unwrap();
        let g = add_group(&mut p, &[r], "grp").unwrap();
        assert_eq!(g, vec![r, 2]);
        // move room0 into group (after room1)
        let np = move_node(&mut p, &[r, 0], &g, 0).unwrap();
        assert_eq!(np, vec![r, 1, 0]);
        assert_eq!(p.room_order, vec![1, 0]);
        // cannot move into another kind
        let s = root_index(&p, ResourceKind::Sprite).unwrap();
        assert!(move_node(&mut p, &np, &[s], 0).is_none());
        // cannot move group into itself
        assert!(move_node(&mut p, &[r, 1], &[r, 1], 0).is_none());
        // delete group deletes its room
        let del = delete_node(&mut p, &[r, 1]);
        assert_eq!(del, vec![(ResourceKind::Room, 0)]);
        assert_eq!(p.room_order, vec![1]);
    }

    #[test]
    fn move_within_same_parent() {
        let mut p = new_project();
        for _ in 0..3 {
            create_resource(&mut p, ResourceKind::Script, &Placement::End);
        }
        let r = root_index(&p, ResourceKind::Script).unwrap();
        // move first to end (pos = len)
        move_node(&mut p, &[r, 0], &[r], 3).unwrap();
        let order: Vec<i32> = tree_order(&p, ResourceKind::Script)
            .iter()
            .map(|x| x.0)
            .collect();
        assert_eq!(order, vec![1, 2, 0]);
        // move last to front
        move_node(&mut p, &[r, 2], &[r], 0).unwrap();
        let order: Vec<i32> = tree_order(&p, ResourceKind::Script)
            .iter()
            .map(|x| x.0)
            .collect();
        assert_eq!(order, vec![0, 1, 2]);
    }

    #[test]
    fn duplicate_and_rename() {
        let mut p = new_project();
        create_resource(&mut p, ResourceKind::Object, &Placement::End);
        rename_resource(&mut p, ResourceKind::Object, 0, "obj_player");
        let path = find_leaf(&p, ResourceKind::Object, 0).unwrap();
        assert_eq!(node(&p.resource_tree, &path).unwrap().name, "obj_player");
        let (i, dp) = duplicate_resource(&mut p, ResourceKind::Object, 0).unwrap();
        assert_eq!(
            res_name(&p, ResourceKind::Object, i),
            Some("obj_player_copy")
        );
        assert_eq!(dp[1], path[1] + 1);
    }
}
