//! Three modelled VST3 hosts (Kör's and Fläns's B-PNL-11: "Three hosts
//! are modelled"): what a host does with the panel's gestures,
//! `beginEdit`, `performEdit` and `endEdit` on its component handler,
//! before its next `process` call. Copied here from Kör's
//! `tests/vst3_host/model.rs` (itself Rödalog v3's, CUBASE-1a) with the
//! code it tests (W-2b; Kör's and Fläns's copies go when they move onto
//! this crate), the models unchanged; their own checks are in
//! `tests/host_models.rs`. Panelwerk's test kit models the same three
//! (`panelwerk_testkit::Model`) over a fake host; these are the real
//! wrapper's.
//!
//! Why it matters: while the host is processing, nice-plug's VST3 wrapper
//! applies none of the panel's sets itself (`context.rs`,
//! `raw_set_parameter_normalized`: only `performEdit`), and its controller
//! reports the old value (`wrapper.rs`, `getParamNormalized`) until the
//! host passes the change back into `process` (Rödalog's
//! `docs/research/cubase-v2-2026-11-13.md`, F1). So what the host passes
//! back, and when, decides whether an edit made in one instant (a click, a
//! double-click) survives.
//!
//! Which host Cubase is, is likely, not proven (that diagnosis, F4): no
//! Cubase was run for it. The models are the diagnosis's three; their
//! sources are in each one's doc, with what is inference marked so.

use std::sync::{Arc, Mutex};

use nice_plug::wrapper::vst3::vst3::Steinberg::Vst::{
    IComponentHandler, IComponentHandlerTrait, IEditController, IEditControllerTrait, ParamID, ParamValue,
};
use nice_plug::wrapper::vst3::vst3::Steinberg::{int32, kResultOk, tresult};
use nice_plug::wrapper::vst3::vst3::{Class, ComPtr, ComWrapper};

/// What a host passes on to `process` of the gestures it's told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    /// **Echo** (Reaper-like): passes each parameter's last `performEdit`
    /// value into the next `process` call, whether or not its gesture has
    /// ended. What VST3 asks of a host (from memory, as in the diagnosis's
    /// F4, not checked here: the host transfers a `performEdit` to the
    /// processor through the next call's parameter changes), and what
    /// Reaper evidently does: Rödalog v2's instant edits worked in the
    /// owner's Reaper (2026-11-11), and the diagnosis's probe modelled
    /// Reaper so.
    Echo,
    /// **Read-back at `endEdit`**: forwards each `performEdit` as
    /// [`Model::Echo`] does, and at `endEdit` reads the controller's value
    /// (`getParamNormalized`) and sends that too, after the performed one,
    /// in the next `process` call, so the processor ends on what the
    /// controller said. Reported of Cubase: Steinberg forum, "Cubase sending
    /// unwanted 0.0 parameter value after endEdit" (Cubase Elements 10.5,
    /// Nuendo 11, in Latch automation mode: a value read from the
    /// controller reached `process()` after `endEdit`;
    /// https://forums.steinberg.net/t/cubase-sending-unwanted-0-0-parameter-value-after-endedit/697244).
    /// Inference: that it forwards the `performEdit`s during the gesture
    /// (VST3 asks it to, and a host that didn't would leave the sound still
    /// while a knob is dragged); and whether Cubase does any of this with
    /// automation writing off. This is
    /// the diagnosis probe's `reads_back_at_end` (Rödalog's
    /// `docs/research/cubase-v2-probe-2026-11-13.md`).
    ReadBack,
    /// **While open**: forwards a parameter's last `performEdit` value only
    /// if its gesture is still open when the next `process` call runs; one
    /// begun, performed and ended between two calls is never forwarded, nor
    /// a `performEdit` outside a gesture. Reported: Steinberg forum,
    /// "Parameter updates get lost between controller and processor using
    /// performEdit" (single `performEdit`s not reaching `process()` in some
    /// hosts, buttons affected; the workaround there was to resend the value
    /// for a few frames;
    /// https://forums.steinberg.net/t/parameter-updates-get-lost-between-controller-and-processor-using-performedit/201936).
    /// Inference: that the edits are lost this way (the thread reports the
    /// loss, not the mechanism), and that Cubase is such a host.
    WhileOpen,
}

/// Every model, for a test that runs under each.
pub const MODELS: [Model; 3] = [Model::Echo, Model::ReadBack, Model::WhileOpen];

/// What the host has gathered for its next `process` call.
#[derive(Default)]
struct Gathered {
    /// The parameters whose gesture is open.
    open: Vec<ParamID>,
    /// The points for the next call, in the order they're to be applied
    /// (each at sample 0): at most one performed value per parameter (its
    /// last), then, for the read-back host, the value read at `endEdit`.
    next: Vec<(ParamID, ParamValue)>,
}

/// Reads the controller's value of a parameter, as the read-back host does
/// at `endEdit`.
type Reader = Box<dyn Fn(ParamID) -> ParamValue + Send + Sync>;

/// A modelled host: what it's told through its component handler
/// ([`handler`]), and what it passes to the next `process` call
/// ([`Self::take`]).
pub struct ModelHost {
    model: Model,
    gathered: Mutex<Gathered>,
    /// What [`Model::ReadBack`] reads at `endEdit`: the controller's
    /// `getParamNormalized` ([`Self::read_from`]), or, in the models' own
    /// tests, a stand-in.
    read: Mutex<Option<Reader>>,
}

impl ModelHost {
    pub fn model(&self) -> Model {
        self.model
    }

    /// Reads what [`Model::ReadBack`] reads at `endEdit` from `controller`.
    /// The handler then holds the controller, as the controller holds it:
    /// the cycle ends when the controller's handler is taken away
    /// (`panel::Panel`'s drop does).
    pub fn read_from(&self, controller: ComPtr<IEditController>) {
        self.read_with(Box::new(move |id| unsafe { controller.getParamNormalized(id) }));
    }

    /// Reads with `read` instead (the models' own tests).
    pub fn read_with(&self, read: Reader) {
        *self.read.lock().unwrap() = Some(read);
    }

    /// The parameter changes for the next `process` call, as (parameter,
    /// value) points at sample 0 in the order given; the host then holds
    /// none.
    pub fn take(&self) -> Vec<(ParamID, ParamValue)> {
        std::mem::take(&mut self.gathered.lock().unwrap().next)
    }

    fn begin(&self, id: ParamID) {
        self.gathered.lock().unwrap().open.push(id);
    }

    fn perform(&self, id: ParamID, value: ParamValue) {
        let mut gathered = self.gathered.lock().unwrap();
        if self.model == Model::WhileOpen && !gathered.open.contains(&id) {
            return;
        }
        // The last value: a later one replaces one not yet passed on.
        gathered.next.retain(|&(p, _)| p != id);
        gathered.next.push((id, value));
    }

    fn end(&self, id: ParamID) {
        // What the controller says as the gesture ends.
        let read = (self.model == Model::ReadBack).then(|| {
            let reader = self.read.lock().unwrap();
            reader.as_ref().expect("the read-back host has a controller to read")(id)
        });
        let mut gathered = self.gathered.lock().unwrap();
        if let Some(at) = gathered.open.iter().position(|&p| p == id) {
            gathered.open.remove(at);
        }
        match (self.model, read) {
            (Model::ReadBack, Some(value)) => gathered.next.push((id, value)),
            (Model::WhileOpen, _) => gathered.next.retain(|&(p, _)| p != id),
            _ => {}
        }
    }
}

/// The host's component handler, telling its [`ModelHost`].
struct Handler(Arc<ModelHost>);

impl Class for Handler {
    type Interfaces = (IComponentHandler,);
}

impl IComponentHandlerTrait for Handler {
    unsafe fn beginEdit(&self, id: ParamID) -> tresult {
        self.0.begin(id);
        kResultOk
    }
    unsafe fn performEdit(&self, id: ParamID, value: ParamValue) -> tresult {
        self.0.perform(id, value);
        kResultOk
    }
    unsafe fn endEdit(&self, id: ParamID) -> tresult {
        self.0.end(id);
        kResultOk
    }
    unsafe fn restartComponent(&self, _flags: int32) -> tresult {
        kResultOk
    }
}

/// A new host of `model`: its component handler, to give the controller,
/// and the host.
pub fn handler(model: Model) -> (ComPtr<IComponentHandler>, Arc<ModelHost>) {
    let host = Arc::new(ModelHost { model, gathered: Mutex::default(), read: Mutex::new(None) });
    (ComWrapper::new(Handler(host.clone())).to_com_ptr::<IComponentHandler>().unwrap(), host)
}
