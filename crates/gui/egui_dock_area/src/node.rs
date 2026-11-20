//! The layout of an area: groups (the host's, e.g. an editor with its tabs), split side by side or
//! one above the other.
//!
//! ```text
//! Split(Horizontal, 0.6)            ┌──────────────┬─────────┐
//! ├─ First:  Group A                │ A            │ B       │
//! └─ Second: Group B                └──────────────┴─────────┘
//! ```
//!
//! A node is found by its [`Path`] from the root: e.g. `[Second]` is `B` above. A group taken out
//! gives its place to the other half of its split.

/// How a split lays out its halves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    /// Side by side: `First` on the left.
    Horizontal,
    /// One above the other: `First` on top.
    Vertical,
}

/// A half of a split.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Half {
    First,
    Second,
}

/// The way from the root to a node: which half at each split.
pub type Path = Vec<Half>;

/// Two nodes, side by side or one above the other.
#[derive(Debug, Clone)]
pub struct Split<G> {
    pub axis: Axis,
    /// The share of the first half, from `0` to `1`.
    pub fraction: f32,
    pub first: Node<G>,
    pub second: Node<G>,
}

/// See the module docs.
#[derive(Debug, Clone)]
pub enum Node<G> {
    Group(G),
    Split(Box<Split<G>>),
}

impl<G> Node<G> {
    /// `first` and `second` split along `axis`, `fraction` of it to `first`.
    #[inline]
    #[must_use]
    pub fn split(axis: Axis, fraction: f32, first: Self, second: Self) -> Self {
        Self::Split(Box::new(Split { axis, fraction, first, second }))
    }

    /// Every group, with its path, top left first.
    #[must_use]
    pub fn groups(&self) -> Vec<(Path, &G)> {
        let mut found = Vec::new();
        self.visit(&mut Vec::new(), &mut |path, group| found.push((path.clone(), group)));
        found
    }

    fn visit<'a>(&'a self, path: &mut Path, found: &mut impl FnMut(&Path, &'a G)) {
        match self {
            Self::Group(group) => found(path, group),
            Self::Split(split) => {
                for (half, node) in [(Half::First, &split.first), (Half::Second, &split.second)] {
                    path.push(half);
                    node.visit(path, found);
                    path.pop();
                }
            }
        }
    }

    /// Every group, top left first.
    pub fn groups_mut(&mut self) -> Vec<&mut G> {
        match self {
            Self::Group(group) => vec![group],
            Self::Split(split) => {
                let mut groups = split.first.groups_mut();
                groups.extend(split.second.groups_mut());
                groups
            }
        }
    }

    /// The node with `group` beside the one at `path`: on its `half` of a new split along `axis`.
    ///
    /// # Errors
    /// The node as it was and `group`, if `path` leads nowhere.
    pub fn put_beside(self, path: &[Half], (axis, half): (Axis, Half), group: G) -> Result<Self, (Self, G)> {
        let Some((&step, rest)) = path.split_first() else {
            let new = Self::Group(group);
            return Ok(match half {
                Half::First => Self::split(axis, 0.5, new, self),
                Half::Second => Self::split(axis, 0.5, self, new),
            });
        };
        let Self::Split(split) = self else { return Err((self, group)) };
        let Split { axis: own_axis, fraction, first, second } = *split;
        let rebuilt = |first, second| Self::split(own_axis, fraction, first, second);
        match step {
            Half::First => match first.put_beside(rest, (axis, half), group) {
                Ok(first) => Ok(rebuilt(first, second)),
                Err((first, group)) => Err((rebuilt(first, second), group)),
            },
            Half::Second => match second.put_beside(rest, (axis, half), group) {
                Ok(second) => Ok(rebuilt(first, second)),
                Err((second, group)) => Err((rebuilt(first, second), group)),
            },
        }
    }

    /// The node with only the groups that `keep` says; a split losing a half gives way to the other.
    /// `None`: no group is kept.
    #[must_use]
    pub fn retain(self, keep: &mut impl FnMut(&G) -> bool) -> Option<Self> {
        match self {
            Self::Group(group) => keep(&group).then_some(Self::Group(group)),
            Self::Split(split) => {
                let Split { axis, fraction, first, second } = *split;
                match (first.retain(keep), second.retain(keep)) {
                    (Some(first), Some(second)) => Some(Self::split(axis, fraction, first, second)),
                    (Some(only), None) | (None, Some(only)) => Some(only),
                    (None, None) => None,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn groups(node: &Node<&'static str>) -> Vec<&'static str> {
        node.groups().into_iter().map(|(_, group)| *group).collect()
    }

    /// A group put beside another splits there, on the side asked; deeper too.
    #[test]
    fn a_group_goes_beside_another() {
        let node = Node::Group("a").put_beside(&[], (Axis::Horizontal, Half::Second), "b").unwrap();
        assert_eq!(groups(&node), ["a", "b"]);
        let node = node.put_beside(&[Half::Second], (Axis::Vertical, Half::First), "c").unwrap();
        assert_eq!(groups(&node), ["a", "c", "b"], "c above b");
        assert_eq!(node.groups()[1].0, [Half::Second, Half::First]);

        let Err((node, group)) = node.put_beside(&[Half::First, Half::First], (Axis::Vertical, Half::First), "d")
        else {
            panic!("a group has no halves")
        };
        assert_eq!((groups(&node), group), (vec!["a", "c", "b"], "d"), "as it was");
    }

    /// A group taken out gives its place to the other half, however deep; none kept, nothing.
    #[test]
    fn a_group_taken_out_gives_way() {
        let inner = Node::split(Axis::Vertical, 0.5, Node::Group("a"), Node::Group("gone"));
        let node = Node::split(Axis::Horizontal, 0.3, inner, Node::Group("b"));
        let node = node.retain(&mut |group| *group != "gone").unwrap();
        assert_eq!(groups(&node), ["a", "b"]);
        assert!(
            matches!(&node, Node::Split(split) if matches!(split.first, Node::Group("a")) && (split.fraction - 0.3).abs() < f32::EPSILON)
        );
        assert!(node.retain(&mut |_| false).is_none());
    }
}
