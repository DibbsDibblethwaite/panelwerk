//! A native VST3 test host (W-2b), copied from Kör's
//! `tests/vst3_host/mod.rs` (itself Rödalog v3's, cut to what B-PNL-11's
//! tests use): a plugin set up and processing as a host makes it, its
//! process calls carrying the host's parameter changes, and the host's ID
//! for a parameter. Here it drives crates.io's nice-plug 0.4.2, unpatched:
//! the consumers' patches are in `wrapper/` and `event_loop/` (W-2a), and
//! none of the ones this touches changes what these tests ask (each
//! consumer's own tests run it under its own copy when it moves onto the
//! crate).
//!
//! Its process calls are made on the thread that created the instance,
//! which nice-plug takes for the main thread. [`Signal`] and [`DEADLINE`]
//! are Kör's and Rödalog's too, natively only: no test here runs nice-plug's
//! main-thread queue on Windows, where it is a hidden window's messages.

#![allow(dead_code)] // Each test binary uses part of it.

pub mod model;
pub mod panel;

use std::ptr;
#[cfg(not(windows))]
use std::sync::{Condvar, Mutex};
#[cfg(not(windows))]
use std::time::Duration;

use nice_plug::wrapper::vst3::vst3::Steinberg::Vst::{
    AudioBusBuffers, AudioBusBuffers__type0, IAudioProcessor, IAudioProcessorTrait, IComponent, IComponentTrait,
    IEditController, IEditControllerTrait, IParamValueQueue, IParamValueQueueTrait, IParameterChanges,
    IParameterChangesTrait, ParamID, ParamValue, ParameterInfo, ProcessData, ProcessModes_, ProcessSetup,
    SymbolicSampleSizes_,
};
use nice_plug::wrapper::vst3::vst3::Steinberg::{int32, kResultOk, tresult};
use nice_plug::wrapper::vst3::vst3::{Class, ComPtr, ComWrapper};

/// The sample rate and largest block the host sets up.
pub const SR: f64 = 48_000.0;
pub const MAX_BLOCK: usize = 512;

/// One parameter's changes in one process call, as (sample, value) points.
struct Queue {
    id: ParamID,
    points: Vec<(int32, ParamValue)>,
}

impl Class for Queue {
    type Interfaces = (IParamValueQueue,);
}

impl IParamValueQueueTrait for Queue {
    unsafe fn getParameterId(&self) -> ParamID {
        self.id
    }
    unsafe fn getPointCount(&self) -> int32 {
        self.points.len() as int32
    }
    unsafe fn getPoint(&self, index: int32, sample_offset: *mut int32, value: *mut ParamValue) -> tresult {
        let (offset, point) = self.points[index as usize];
        // SAFETY: the wrapper's out-pointers, valid for the call.
        unsafe {
            *sample_offset = offset;
            *value = point;
        }
        kResultOk
    }
    unsafe fn addPoint(&self, _: int32, _: ParamValue, _: *mut int32) -> tresult {
        unreachable!("the plugin adds no points to its input")
    }
}

/// The host's parameter changes for one process call.
struct Changes(Vec<ComPtr<IParamValueQueue>>);

impl Class for Changes {
    type Interfaces = (IParameterChanges,);
}

impl IParameterChangesTrait for Changes {
    unsafe fn getParameterCount(&self) -> int32 {
        self.0.len() as int32
    }
    unsafe fn getParameterData(&self, index: int32) -> *mut IParamValueQueue {
        self.0[index as usize].as_ptr()
    }
    unsafe fn addParameterData(&self, _: *const ParamID, _: *mut int32) -> *mut IParamValueQueue {
        unreachable!("the plugin adds nothing to its input")
    }
}

/// One plugin instance, as a VST3 host sees it: one object behind the
/// component, controller and processor interfaces (nice-plug's wrapper).
pub struct Vst3 {
    pub component: ComPtr<IComponent>,
    pub controller: ComPtr<IEditController>,
    pub processor: ComPtr<IAudioProcessor>,
}

impl Vst3 {
    /// `component`, already initialised, set up for 48 kHz, made active and
    /// processing, as a host does before its first process call.
    pub fn set_up(component: ComPtr<IComponent>) -> Self {
        unsafe {
            let controller: ComPtr<IEditController> = component.cast().unwrap();
            let processor: ComPtr<IAudioProcessor> = component.cast().unwrap();
            let mut setup = ProcessSetup {
                processMode: ProcessModes_::kRealtime as int32,
                symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                maxSamplesPerBlock: MAX_BLOCK as int32,
                sampleRate: SR,
            };
            assert_eq!(processor.setupProcessing(&mut setup), kResultOk);
            assert_eq!(component.setActive(1), kResultOk);
            assert_eq!(processor.setProcessing(1), kResultOk);
            Self { component, controller, processor }
        }
    }

    /// The ID of the host parameter titled `name`.
    pub fn param(&self, name: &str) -> ParamID {
        unsafe {
            (0..self.controller.getParameterCount())
                .map(|i| {
                    let mut p: ParameterInfo = std::mem::zeroed();
                    assert_eq!(self.controller.getParameterInfo(i, &mut p), kResultOk);
                    p
                })
                .find(|p| {
                    let len = p.title.iter().position(|&c| c == 0).unwrap_or(p.title.len());
                    String::from_utf16_lossy(&p.title[..len]) == name
                })
                .unwrap_or_else(|| panic!("no {name} parameter"))
                .id
        }
    }

    /// A process call rendering `samples` of stereo, carrying `changes`,
    /// each (parameter, value) a single point at sample 0, one queue per
    /// parameter (in the order each first appears; a parameter's points in
    /// the order given), as a host sends them.
    pub fn process(&self, changes: &[(ParamID, ParamValue)], samples: usize) {
        let mut ids: Vec<ParamID> = Vec::new();
        for &(id, _) in changes {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        let queues = ids
            .iter()
            .map(|&id| {
                let points = changes.iter().filter(|p| p.0 == id).map(|p| (0, p.1)).collect();
                ComWrapper::new(Queue { id, points }).to_com_ptr::<IParamValueQueue>().unwrap()
            })
            .collect();
        let changes = ComWrapper::new(Changes(queues)).to_com_ptr::<IParameterChanges>().unwrap();
        let mut left = vec![0.0f32; samples];
        let mut right = vec![0.0f32; samples];
        let mut channels = [left.as_mut_ptr(), right.as_mut_ptr()];
        let mut bus = AudioBusBuffers {
            numChannels: 2,
            silenceFlags: 0,
            __field0: AudioBusBuffers__type0 { channelBuffers32: channels.as_mut_ptr() },
        };
        let mut data = ProcessData {
            processMode: ProcessModes_::kRealtime as int32,
            symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
            numSamples: samples as int32,
            numInputs: 0,
            numOutputs: 1,
            inputs: ptr::null_mut(),
            outputs: &mut bus,
            inputParameterChanges: changes.as_ptr(),
            outputParameterChanges: ptr::null_mut(),
            inputEvents: ptr::null_mut(),
            outputEvents: ptr::null_mut(),
            processContext: ptr::null_mut(),
        };
        assert_eq!(unsafe { self.processor.process(&mut data) }, kResultOk);
    }
}

/// Natively, one more instance of the stand-in plugin `P`, never released:
/// an instance released just as nice-plug's shared worker thread finishes
/// its last task can be dropped there, and if it held the worker's last
/// handle the worker joins itself and aborts (found in Rödalog, its
/// `vendor/PATCHES.md`, Known upstream issues; crates.io's 0.4.2 is the
/// same). Made before the stand-in under test. On Windows each instance's
/// tasks are its own window's messages, so there is nothing to keep.
#[cfg(not(windows))]
pub fn keep_another<P: nice_plug::wrapper::vst3::Vst3Plugin>() {
    let kept = ComWrapper::new(nice_plug::wrapper::vst3::Wrapper::<P>::new()).to_com_ptr::<IComponent>().unwrap();
    std::mem::forget(kept);
}

/// How long a test waits for the plugin's queued main-thread work before it
/// fails: far longer than a trip through the queue takes (well under a
/// millisecond), so only work that never comes waits it out; a bound, so a
/// fault fails rather than hangs (TEST-WAIT: no fixed waits in tests).
#[cfg(not(windows))]
pub const DEADLINE: Duration = Duration::from_secs(5);

/// A count raised from any thread, waited on with [`DEADLINE`].
#[cfg(not(windows))]
pub struct Signal {
    count: Mutex<usize>,
    raised: Condvar,
}

#[cfg(not(windows))]
impl Signal {
    pub const fn new() -> Self {
        Self { count: Mutex::new(0), raised: Condvar::new() }
    }

    pub fn raise(&self) {
        *self.count.lock().unwrap() += 1;
        self.raised.notify_all();
    }

    /// How many times it has been raised.
    pub fn count(&self) -> usize {
        *self.count.lock().unwrap()
    }

    /// Waits until the count is at least `n`, at most [`DEADLINE`];
    /// whether it is.
    pub fn wait_for(&self, n: usize) -> bool {
        let count = self.count.lock().unwrap();
        *self.raised.wait_timeout_while(count, DEADLINE, |c| *c < n).unwrap().0 >= n
    }
}
