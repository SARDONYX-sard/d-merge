use dashmap::DashMap;

use crate::behaviors::tasks::fnis::collect::owned::OwnedFnisInjection;

/// This struct represents all FNIS injection data for a Skyrim Data directory, grouped per **actor name**.
///
/// This generates patches for each template target and combines them into a `BorrowedPatch` (the Nemesis patch intermediate structure).
///
/// # Structure
///
/// ## **Key** (`String`): the actor's folder name under `meshes/actors/`.
///   Examples:
///   - `"character"`
///   - `"cow"`
///   - `"wolf"`
///
/// ## **Value** (`Vec<OwnedFnisInjection>`): all FNIS injection data collected
///   for that actor. Each element corresponds to a single FNIS mod namespace
///   found under `meshes/actors/<actor>/animations/<namespace>/`.
///
/// # Example layout in a typical Skyrim Data directory
///
/// ```txt
/// Data/
/// └── meshes/
///     └── actors/
///         ├── character/
///         │   └── animations/
///         │       ├── FNISFlyer/
///         │       └── FNISRunner/
///         └── cow/
///             └── animations/
///                 └── FNISFlyer/
///
/// ActorFnisInjections structure (conceptual):
/// {
///     "character": [
///         OwnedFnisInjection { namespace: "FNISFlyer", ... },
///         OwnedFnisInjection { namespace: "FNISRunner", ... },
///     ],
///     "cow": [
///         OwnedFnisInjection { namespace: "FNISFlyer", ... },
///     ],
/// }
/// ```
///
/// # Notes
/// - This type is **thread-safe**: multiple threads can concurrently insert
///   injections into different actors.
/// - The order of injections in the Vec is **not guaranteed**.
/// - Intended for use in constructing FNIS patch data per actor for Nemesis/D-merge.
#[derive(Debug, Default)]
pub(crate) struct OwnedActorFnisMap(DashMap<String, Vec<OwnedFnisInjection>>);

impl OwnedActorFnisMap {
    /// Creates a new, empty collection.
    pub(crate) fn new() -> Self {
        Self(DashMap::new())
    }

    /// Returns a reference to the inner DashMap
    pub(crate) const fn inner(&self) -> &DashMap<String, Vec<OwnedFnisInjection>> {
        &self.0
    }
}
