//! What is docked where: the host's area, and the windows of their own groups went out to.
//!
//! ```text
//! main                         windows
//! ┌──────────────┬─────────┐   ┌ #1 ────────┐
//! │ group A      │ group B │   │ group C    │   <- an OS window, opened by a tab dropped out of
//! └──────────────┴─────────┘   └────────────┘      the main window; it closes once empty
//! ```
//!
//! A group is the host's (e.g. an editor and its tabs); this only places them. The host moves its
//! tabs between them when one is dropped: see [`crate::Dropped`].

use egui::Rect;

use crate::node::{Axis, Half, Node, Path};

/// An area groups are docked in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Area {
    /// The host's, where [`crate::DockArea::show_inside`] draws.
    Main,
    /// A window of its own, by its number.
    Window(u64),
}

/// A group in an area.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Place {
    pub area: Area,
    pub path: Path,
}

impl Default for Area {
    #[inline]
    fn default() -> Self {
        Self::Main
    }
}

impl Place {
    #[inline]
    #[must_use]
    pub const fn new(area: Area, path: Path) -> Self {
        Self { area, path }
    }

    /// Its half `half`, if it is a split.
    #[must_use]
    pub fn child(&self, half: Half) -> Self {
        let mut path = self.path.clone();
        path.push(half);
        Self { area: self.area, path }
    }
}

/// Where a dragged tab was dropped.
#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    /// Among the tabs of a group: at this index of its bar.
    Join(Place, usize),
    /// Beside a group: on its `Half` of a split along `Axis`, a group of its own.
    Beside(Place, Axis, Half),
    /// Out of every area: a window of its own, there on the screen.
    NewWindow(Rect),
}

/// A window of its own: its groups, and where it opened.
#[derive(Debug, Clone)]
pub struct Window<G> {
    pub id: u64,
    pub node: Node<G>,
    /// Where it opened on the screen: it is moved and sized by the user then.
    pub opened_at: Rect,
}

/// See the module docs. `P`: what the host drags (e.g. which tab), see [`crate::Dropped`].
#[derive(Debug)]
pub struct Dock<G, P> {
    /// Always `Some` but while a change rebuilds it.
    main: Option<Node<G>>,
    pub windows: Vec<Window<G>>,
    next_window: u64,
    pub(crate) runtime: crate::view::Runtime<P>,
}

/// One group, the default one, in the main area.
impl<G: Default, P> Default for Dock<G, P> {
    #[inline]
    fn default() -> Self {
        Self::new(G::default())
    }
}

impl<G, P> Dock<G, P> {
    /// One group in the main area.
    #[inline]
    #[must_use]
    pub fn new(group: G) -> Self {
        let runtime = crate::view::Runtime::default();
        Self { main: Some(Node::Group(group)), windows: Vec::new(), next_window: 1, runtime }
    }

    /// The main area's layout.
    #[inline]
    #[must_use]
    pub fn main(&self) -> &Node<G> {
        self.main.as_ref().unwrap_or_else(|| unreachable!("rebuilt in place"))
    }

    pub(crate) fn main_mut(&mut self) -> &mut Node<G> {
        self.main.as_mut().unwrap_or_else(|| unreachable!("rebuilt in place"))
    }

    /// Every group, with its place: the main area's first, then each window's.
    #[must_use]
    pub fn groups(&self) -> Vec<(Place, &G)> {
        let main = self.main().groups().into_iter().map(|(path, group)| (Place::new(Area::Main, path), group));
        let windows = self.windows.iter().flat_map(|window| {
            let area = Area::Window(window.id);
            window.node.groups().into_iter().map(move |(path, group)| (Place::new(area, path), group))
        });
        main.chain(windows).collect()
    }

    /// Every group.
    pub fn groups_mut(&mut self) -> Vec<&mut G> {
        let Self { main, windows, .. } = self;
        let mut groups = main.as_mut().map(Node::groups_mut).unwrap_or_default();
        for window in windows {
            groups.extend(window.node.groups_mut());
        }
        groups
    }

    /// Puts `group` beside the group at `place`, on its `half` of a split along `axis`.
    ///
    /// # Errors
    /// `group` back, if there is no group at `place`.
    pub fn put_beside(&mut self, place: &Place, side: (Axis, Half), group: G) -> Result<(), G> {
        let put = |node: Node<G>, group| match node.put_beside(&place.path, side, group) {
            Ok(node) => (node, Ok(())),
            Err((node, group)) => (node, Err(group)),
        };
        match place.area {
            Area::Main => {
                let (main, done) = put(self.main.take().unwrap_or_else(|| unreachable!("rebuilt in place")), group);
                self.main = Some(main);
                done
            }
            Area::Window(id) => {
                let Some(at) = self.windows.iter().position(|window| window.id == id) else { return Err(group) };
                let Window { id, node, opened_at } = self.windows.remove(at);
                let (node, done) = put(node, group);
                self.windows.insert(at, Window { id, node, opened_at });
                done
            }
        }
    }

    /// Opens `group` in a window of its own, at `at` on the screen. Returns its number.
    pub fn open_window(&mut self, group: G, at: Rect) -> u64 {
        let id = self.next_window;
        self.next_window += 1;
        self.windows.push(Window { id, node: Node::Group(group), opened_at: at });
        id
    }

    /// Keeps the groups that `keep` says, wherever they are; a window left with none closes. The
    /// main area keeps one at least: if `keep` says none, it is as it was.
    pub fn retain(&mut self, mut keep: impl FnMut(&G) -> bool) {
        if self.main().groups().into_iter().any(|(_, group)| keep(group)) {
            self.main = self.main.take().and_then(|main| main.retain(&mut keep));
        }
        let windows = std::mem::take(&mut self.windows);
        self.windows = windows
            .into_iter()
            .filter_map(|window| {
                let Window { id, node, opened_at } = window;
                node.retain(&mut keep).map(|node| Window { id, node, opened_at })
            })
            .collect();
    }

    /// Window `id` was closed by the user: its groups, for the host to take in.
    pub fn close_window(&mut self, id: u64) -> Option<Node<G>> {
        let at = self.windows.iter().position(|window| window.id == id)?;
        Some(self.windows.remove(at).node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(dock: &Dock<&'static str, ()>) -> Vec<(Area, &'static str)> {
        dock.groups().into_iter().map(|(place, group)| (place.area, *group)).collect()
    }

    /// Beside a group of the main area or of a window; a window left empty closes; the main area
    /// keeps a group.
    #[test]
    fn groups_go_beside_others_and_windows_close_empty() {
        let mut dock: Dock<&str, ()> = Dock::new("a");
        dock.put_beside(&Place::default(), (Axis::Horizontal, Half::Second), "b").unwrap();
        let window = Area::Window(dock.open_window("c", Rect::ZERO));
        dock.put_beside(&Place::new(window, Vec::new()), (Axis::Vertical, Half::Second), "d").unwrap();
        assert_eq!(names(&dock), [(Area::Main, "a"), (Area::Main, "b"), (window, "c"), (window, "d")]);
        assert_eq!(
            dock.put_beside(&Place::new(Area::Window(9), Vec::new()), (Axis::Vertical, Half::First), "e"),
            Err("e")
        );

        dock.retain(|group| !matches!(*group, "c" | "d" | "a"));
        assert_eq!(names(&dock), [(Area::Main, "b")], "the window closed, empty");
        dock.retain(|_| false);
        assert_eq!(names(&dock), [(Area::Main, "b")], "the main area keeps one");
    }

    #[test]
    fn a_closed_window_gives_its_groups_back() {
        let mut dock: Dock<&str, ()> = Dock::new("a");
        let id = dock.open_window("b", Rect::ZERO);
        let node = dock.close_window(id).unwrap();
        assert!(matches!(node, Node::Group("b")));
        assert!(dock.windows.is_empty());
    }
}
