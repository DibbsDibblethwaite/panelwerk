//! Panelwerk's held edits on nice-plug (W-2b; Ian, 2026-11-23, PW-BUILT,
//! point 1 (a)): the framework's side of `panelwerk_panel::hold`, once for
//! the family, where Kör and Fläns each carried a copy.
//!
//! - [`ParamMap`] and [`Adapter`]: the panel's `ParamHost` over a plugin's
//!   parameters and its window's `GuiContext`, `shows` comparing plain
//!   values.
//! - [`HeldEdits`]: one window's held edits, owned by its app; each frame
//!   ends the edits whose values have arrived, gives the panel its host,
//!   and asks for another frame while an edit waits.
//! - [`Leaving`] and [`Handle`]: the window's handle wrapped, so its close
//!   and its hide take the edits still open and hand them on as an
//!   [`EndEdits`] task, which [`queue_later`] queues for the main thread:
//!   never ended inside the host's call (`queue_later` says why).
//!
//! This is the one Panelwerk crate that depends on nice-plug (exactly
//! 0.4.2, which each consumer redirects to its own patched copy), and the
//! one with `unsafe`, in its `adapter` module only (`CLAUDE.md`, Layout and
//! rule 15; `tests/manifest.rs` checks both). Nothing here runs on the
//! audio thread (rule 3): the panel's thread, and the main thread for
//! [`EndEdits::run`].
//!
//! **Adopting it**, in a plugin whose editor makes its app in each
//! `Editor::spawn` (as Kör's and Fläns's do):
//! - the plugin's `BackgroundTask` takes an [`EndEdits`] (`From`), and its
//!   task executor runs it ([`EndEdits::run`]);
//! - `Plugin::editor` gives the editor [`queue_later`] of its executor;
//! - `spawn` makes the app's [`HeldEdits`], takes [`HeldEdits::leaving`]
//!   with the GUI context and the `queue_later` ends, makes the window, and
//!   returns the window's handle wrapped with [`Leaving::around`];
//! - the app's `build` calls [`HeldEdits::open`], and its `ui` draws the
//!   panel inside [`HeldEdits::frame`], with the host it gives (wrapped in
//!   the consumer's own, for its zoom and its extras).
//!
//! ```
//! use std::sync::Arc;
//! use nice_plug::prelude::*;
//! use panelwerk_nice::{queue_later, EndEdits, HeldEdits, Ends};
//! use panelwerk_panel::egui;
//!
//! #[derive(Params)]
//! struct Knobs {
//!     #[id = "on"]
//!     on: BoolParam,
//! }
//!
//! /// The plugin's main-thread work: here, only the edits a closing
//! /// window left open.
//! enum MainTask {
//!     EndEdits(EndEdits),
//! }
//! impl From<EndEdits> for MainTask {
//!     fn from(task: EndEdits) -> Self {
//!         MainTask::EndEdits(task)
//!     }
//! }
//! /// The plugin's task executor (`Plugin::task_executor`).
//! fn run_main_task(task: MainTask) {
//!     match task {
//!         MainTask::EndEdits(ends) => ends.run(),
//!     }
//! }
//! /// In `Plugin::editor(&mut self, executor)`, for the editor to keep.
//! fn ends_for<P: Plugin<BackgroundTask = MainTask>>(executor: AsyncExecutor<P>) -> Ends {
//!     queue_later(executor)
//! }
//!
//! // In `Editor::spawn`: the app's held edits, made before its window.
//! let params = Arc::new(Knobs { on: BoolParam::new("On", false) });
//! let edits = HeldEdits::new(params);
//! // ... `edits.leaving(gui_context, ends).around(handle)`, and in the
//! // app's `build`, `edits.open(gui_context)`. In its `ui`:
//! let mut window = panelwerk_testkit::Window::new(egui::vec2(100.0, 100.0));
//! window.idle(&mut |ui| {
//!     let drawn = edits.frame(ui, |host| {
//!         // The panel, drawn with `host` (here, before `open`, not drawn).
//!         host.value("on")
//!     });
//!     assert_eq!(drawn, None, "nothing is drawn before the window has opened");
//! });
//! # let _ = (run_main_task, ends_for::<Never>);
//! # #[derive(Default)]
//! # struct Never;
//! # impl Plugin for Never {
//! #     const NAME: &'static str = ""; const VENDOR: &'static str = ""; const URL: &'static str = "";
//! #     const EMAIL: &'static str = ""; const VERSION: &'static str = "";
//! #     const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[];
//! #     type SysExMessage = (); type BackgroundTask = MainTask; type Editor = ();
//! #     fn params(&self) -> Arc<dyn Params> { unreachable!() }
//! #     fn process(&mut self, _: &mut Buffer, _: &mut AuxiliaryBuffers, _: &mut impl ProcessContext<Self>) -> ProcessStatus { unreachable!() }
//! # }
//! ```

// `deny`, not `forbid` as in the other crates, so that the adapter's one
// module can allow it (rule 15's exception, Ian, 2026-11-23, PW-BUILT,
// point 1 (a)); `tests/manifest.rs` checks that it is that module alone.
#![deny(unsafe_code)]
#![deny(missing_docs)]

#[allow(unsafe_code)]
mod adapter;
mod window;

pub use adapter::{Adapter, EndEdits, ParamMap};
pub use window::{queue_later, Ends, Handle, HeldEdits, Leaving};
