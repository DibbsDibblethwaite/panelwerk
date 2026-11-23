//! The crate's one module with `unsafe` (CLAUDE.md rule 15; Ian,
//! 2026-11-23, PW-BUILT, point 1 (a)): every call that follows a
//! [`ParamPtr`] or tells the host through [`GuiContext`]'s `raw_*` setters,
//! the same calls Kör's and Fläns's adapters make (their `app.rs`, `Raw`;
//! their `lib.rs`, `run_main_task`). Everything outside this module reaches
//! the plugin through the safe types here.
//!
//! **Why it is sound.** nice-plug's `ParamPtr` methods are `unsafe` only
//! because the pointer must still point at a live parameter, and a
//! `ParamPtr` can be made by safe code from any pointer. So no pointer here
//! comes from a caller: a [`ParamMap`] takes them from
//! [`Params::param_map`] of the `Arc<dyn Params>` it is given, and keeps
//! that `Arc` for as long as it lives. `Params` is an `unsafe trait` whose
//! contract is that `param_map`'s pointers stay valid while the object
//! does, and nothing hands out a pointer from a map apart from the map
//! itself (no public accessor), so every pointer followed here points into
//! an object this module is keeping alive. The copies in Kör and Fläns
//! relied on the plugin outliving its window and its tasks instead (true,
//! but the plugin's to keep): the `Arc` makes it the crate's, at the cost
//! of one reference count. `GuiContext`'s `raw_*` setters are `unsafe`, in
//! nice-plug's words, "mostly for API reasons": the wrapper looks the
//! pointer up among its plugin's parameters itself. They are given only
//! pointers from a map, as above.

use std::collections::HashMap;
use std::sync::Arc;

use nice_plug::context::gui::GuiContext;
use nice_plug::prelude::{ParamPtr, Params};
use panelwerk_panel::ParamHost;

/// A plugin's parameters by string ID, kept alive with the map: what
/// [`Adapter`] and [`EndEdits`] follow. Cheap to clone (one reference
/// count), so a window's app and its handle can each hold one.
///
/// ```
/// use std::sync::Arc;
/// use nice_plug::prelude::*;
/// use panelwerk_nice::ParamMap;
///
/// #[derive(Params)]
/// struct Knobs {
///     #[id = "mix"]
///     mix: FloatParam,
/// }
///
/// let params = Arc::new(Knobs { mix: FloatParam::new("Mix", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }) });
/// let map = ParamMap::new(params);
/// assert!(map.has("mix"));
/// assert!(!map.has("rate"));
/// ```
#[derive(Clone)]
pub struct ParamMap {
    inner: Arc<Inner>,
}

struct Inner {
    /// The object the pointers point into, kept alive while any clone of
    /// the map, or a task holding one, is (the module doc says why).
    _params: Arc<dyn Params>,
    by_id: HashMap<String, ParamPtr>,
}

impl ParamMap {
    /// Every parameter of `params`, by its string ID (`#[id = "…"]`).
    pub fn new(params: Arc<dyn Params>) -> Self {
        let by_id = params.param_map().into_iter().map(|(id, ptr, _)| (id, ptr)).collect();
        Self { inner: Arc::new(Inner { _params: params, by_id }) }
    }

    /// Whether the plugin has a parameter `id`.
    pub fn has(&self, id: &str) -> bool {
        self.inner.by_id.contains_key(id)
    }

    /// Parameter `id`'s pointer. Panics if the plugin has none: the panel
    /// names only parameters that exist (each consumer's tests check its
    /// panel's IDs), as Kör's and Fläns's `Raw::ptr`.
    fn ptr(&self, id: &str) -> ParamPtr {
        self.inner.by_id[id]
    }
}

/// The panel's [`ParamHost`] over nice-plug: values from the plugin's
/// parameters, gestures to the host through the window's [`GuiContext`].
/// Kör's and Fläns's `Raw`. The panel's thread only, as `GuiContext` asks.
/// A window with held edits gives the panel this through
/// [`HeldEdits::frame`](crate::HeldEdits::frame), never directly: there
/// `begin`, `set` and `end` pass through the held edits first.
///
/// Every call panics on a parameter ID the plugin doesn't have.
pub struct Adapter<'a> {
    map: &'a ParamMap,
    gui: &'a GuiContext,
}

impl<'a> Adapter<'a> {
    /// The adapter for `map`'s parameters, telling the host through `gui`.
    pub fn new(map: &'a ParamMap, gui: &'a GuiContext) -> Self {
        Self { map, gui }
    }
}

impl ParamHost for Adapter<'_> {
    /// The unmodulated value, so the panel shows what the user set, not
    /// what a host's modulation makes of it.
    fn value(&self, id: &str) -> f32 {
        // SAFETY: a pointer from `self.map`, which keeps its parameters
        // alive (the module doc).
        unsafe { self.map.ptr(id).unmodulated_normalized_value() }
    }
    fn default_value(&self, id: &str) -> f32 {
        // SAFETY: as in `value`.
        unsafe { self.map.ptr(id).default_normalized_value() }
    }
    /// The unmodulated value's text with its unit, as the host shows it.
    fn text(&self, id: &str) -> String {
        let p = self.map.ptr(id);
        // SAFETY: as in `value`.
        unsafe { p.normalized_value_to_string(p.unmodulated_normalized_value(), true) }
    }
    fn begin(&self, id: &str) {
        // SAFETY: a pointer to one of the plugin's own parameters, kept
        // alive by `self.map`; the wrapper looks it up itself (the module
        // doc).
        unsafe { self.gui.raw_begin_set_parameter(self.map.ptr(id)) }
    }
    fn set(&self, id: &str, normalized: f32) {
        // SAFETY: as in `begin`.
        unsafe { self.gui.raw_set_parameter_normalized(self.map.ptr(id), normalized) }
    }
    fn end(&self, id: &str) {
        // SAFETY: as in `begin`.
        unsafe { self.gui.raw_end_set_parameter(self.map.ptr(id)) }
    }
    /// Whether the plugin has the value an edit set, as plain values:
    /// nice-plug keeps a parameter's plain value and works its normalised
    /// one out from that again, so a stepped value, or a float's after the
    /// round trip, may not equal the normalised value set, and comparing
    /// those (the trait's default) would end such an edit only at
    /// `gesture::PENDING_FOR` (Panelwerk's `hold.rs`, "Wiring it in"), and
    /// join a second edit of it before then into the same gesture.
    fn shows(&self, id: &str, normalized: f32) -> bool {
        let p = self.map.ptr(id);
        // SAFETY: as in `value`.
        unsafe { p.unmodulated_plain_value() == p.preview_plain(normalized) }
    }
}

/// The task that ends, on the main thread, the edits a window left open as
/// it closed or was hidden ([`Leaving`](crate::Leaving)): Kör's and Fläns's
/// `MainTask::EndEdits`. A consumer's `Plugin::BackgroundTask` takes it
/// (`From<EndEdits>`, for [`queue_later`](crate::queue_later)), and its
/// task executor calls [`EndEdits::run`].
pub struct EndEdits {
    gui: GuiContext,
    map: ParamMap,
    /// The edits' parameters, in the order begun, each with its ID for
    /// [`EndEdits::ids`]; the pointers looked up as the window went, as in
    /// Kör's and Fläns's `Leaving::gone`.
    open: Vec<(String, ParamPtr)>,
}

impl EndEdits {
    /// The task ending `open` (string IDs of `map`'s parameters, in the
    /// order begun) through `gui`. Panics on an ID `map` doesn't have.
    pub(crate) fn new(gui: GuiContext, map: ParamMap, open: Vec<String>) -> Self {
        let open = open.into_iter().map(|id| (id.clone(), map.ptr(&id))).collect();
        Self { gui, map, open }
    }

    /// The parameters whose edits it ends, by string ID, in the order begun
    /// (for a consumer's tests).
    pub fn ids(&self) -> Vec<&str> {
        self.open.iter().map(|(id, _)| id.as_str()).collect()
    }

    /// Tells the host each edit has ended, in the order begun. On the main
    /// thread, from the plugin's task executor: never inside the host's
    /// call that closes or hides the window ([`queue_later`](crate::queue_later)
    /// says why).
    pub fn run(self) {
        for (_, ptr) in self.open {
            // SAFETY: a pointer from `self.map`, which this task holds, so
            // the parameters are alive however late the task runs (the
            // module doc).
            unsafe { self.gui.raw_end_set_parameter(ptr) };
        }
        drop(self.map);
    }
}
