//! FMOD bindings for the Sludge framework. This crate both contains a high-level
//! Rust interface to a subset of the FMOD Core/Studio APIs, as well as bindings
//! to those Rust interfaces through Lua by way of Sludge's module registration
//! API.

use {
    hv_core::{
        engine::{Engine, LuaExt, LuaResource},
        plugins::Plugin,
        prelude::*,
    },
    hv_fmod_sys::*,
    lazy_static::lazy_static,
    regex::Regex,
    std::{
        ffi::CString,
        ptr, str,
        sync::mpsc::{Receiver, Sender},
        sync::Arc,
    },
};

pub mod bank;
pub mod event;

use std::sync::{Mutex, RwLock};

pub use bank::*;
pub use event::*;
use hibitset::{AtomicBitSet, DrainableBitSet};
use thunderdome::{Arena, Index};

trait CheckError {
    fn check_err(self) -> Result<()>;
}

impl CheckError for FMOD_RESULT {
    fn check_err(self) -> Result<()> {
        if self == FMOD_RESULT_FMOD_OK {
            return Ok(());
        }

        match self {
            FMOD_RESULT_FMOD_ERR_ALREADY_LOCKED => bail!("FMOD_RESULT_FMOD_ERR_ALREADY_LOCKED"),
            FMOD_RESULT_FMOD_ERR_BADCOMMAND => bail!("FMOD_RESULT_FMOD_ERR_BADCOMMAND"),
            FMOD_RESULT_FMOD_ERR_CHANNEL_ALLOC => bail!("FMOD_RESULT_FMOD_ERR_CHANNEL_ALLOC"),
            FMOD_RESULT_FMOD_ERR_CHANNEL_STOLEN => bail!("FMOD_RESULT_FMOD_ERR_CHANNEL_STOLEN"),
            FMOD_RESULT_FMOD_ERR_DMA => bail!("FMOD_RESULT_FMOD_ERR_DMA"),
            FMOD_RESULT_FMOD_ERR_DSP_CONNECTION => bail!("FMOD_RESULT_FMOD_ERR_DSP_CONNECTION"),
            FMOD_RESULT_FMOD_ERR_DSP_DONTPROCESS => bail!("FMOD_RESULT_FMOD_ERR_DSP_DONTPROCESS"),
            FMOD_RESULT_FMOD_ERR_DSP_FORMAT => bail!("FMOD_RESULT_FMOD_ERR_DSP_FORMAT"),
            FMOD_RESULT_FMOD_ERR_DSP_INUSE => bail!("FMOD_RESULT_FMOD_ERR_DSP_INUSE"),
            FMOD_RESULT_FMOD_ERR_DSP_NOTFOUND => bail!("FMOD_RESULT_FMOD_ERR_DSP_NOTFOUND"),
            FMOD_RESULT_FMOD_ERR_DSP_RESERVED => bail!("FMOD_RESULT_FMOD_ERR_DSP_RESERVED"),
            FMOD_RESULT_FMOD_ERR_DSP_SILENCE => bail!("FMOD_RESULT_FMOD_ERR_DSP_SILENCE"),
            FMOD_RESULT_FMOD_ERR_DSP_TYPE => bail!("FMOD_RESULT_FMOD_ERR_DSP_TYPE"),
            FMOD_RESULT_FMOD_ERR_EVENT_ALREADY_LOADED => {
                bail!("FMOD_RESULT_FMOD_ERR_EVENT_ALREADY_LOADED")
            }
            FMOD_RESULT_FMOD_ERR_EVENT_LIVEUPDATE_BUSY => {
                bail!("FMOD_RESULT_FMOD_ERR_EVENT_LIVEUPDATE_BUSY")
            }
            FMOD_RESULT_FMOD_ERR_EVENT_LIVEUPDATE_MISMATCH => {
                bail!("FMOD_RESULT_FMOD_ERR_EVENT_LIVEUPDATE_MISMATCH")
            }
            FMOD_RESULT_FMOD_ERR_EVENT_LIVEUPDATE_TIMEOUT => {
                bail!("FMOD_RESULT_FMOD_ERR_EVENT_LIVEUPDATE_TIMEOUT")
            }
            FMOD_RESULT_FMOD_ERR_EVENT_NOTFOUND => bail!("FMOD_RESULT_FMOD_ERR_EVENT_NOTFOUND"),
            FMOD_RESULT_FMOD_ERR_FILE_BAD => bail!("FMOD_RESULT_FMOD_ERR_FILE_BAD"),
            FMOD_RESULT_FMOD_ERR_FILE_COULDNOTSEEK => {
                bail!("FMOD_RESULT_FMOD_ERR_FILE_COULDNOTSEEK")
            }
            FMOD_RESULT_FMOD_ERR_FILE_DISKEJECTED => bail!("FMOD_RESULT_FMOD_ERR_FILE_DISKEJECTED"),
            FMOD_RESULT_FMOD_ERR_FILE_ENDOFDATA => bail!("FMOD_RESULT_FMOD_ERR_FILE_ENDOFDATA"),
            FMOD_RESULT_FMOD_ERR_FILE_EOF => bail!("FMOD_RESULT_FMOD_ERR_FILE_EOF"),
            FMOD_RESULT_FMOD_ERR_FILE_NOTFOUND => bail!("FMOD_RESULT_FMOD_ERR_FILE_NOTFOUND"),
            FMOD_RESULT_FMOD_ERR_FORMAT => bail!("FMOD_RESULT_FMOD_ERR_FORMAT"),
            FMOD_RESULT_FMOD_ERR_HEADER_MISMATCH => bail!("FMOD_RESULT_FMOD_ERR_HEADER_MISMATCH"),
            FMOD_RESULT_FMOD_ERR_HTTP => bail!("FMOD_RESULT_FMOD_ERR_HTTP"),
            FMOD_RESULT_FMOD_ERR_HTTP_ACCESS => bail!("FMOD_RESULT_FMOD_ERR_HTTP_ACCESS"),
            FMOD_RESULT_FMOD_ERR_HTTP_PROXY_AUTH => bail!("FMOD_RESULT_FMOD_ERR_HTTP_PROXY_AUTH"),
            FMOD_RESULT_FMOD_ERR_HTTP_SERVER_ERROR => {
                bail!("FMOD_RESULT_FMOD_ERR_HTTP_SERVER_ERROR")
            }
            FMOD_RESULT_FMOD_ERR_HTTP_TIMEOUT => bail!("FMOD_RESULT_FMOD_ERR_HTTP_TIMEOUT"),
            FMOD_RESULT_FMOD_ERR_INITIALIZATION => bail!("FMOD_RESULT_FMOD_ERR_INITIALIZATION"),
            FMOD_RESULT_FMOD_ERR_INITIALIZED => bail!("FMOD_RESULT_FMOD_ERR_INITIALIZED"),
            FMOD_RESULT_FMOD_ERR_INTERNAL => bail!("FMOD_RESULT_FMOD_ERR_INTERNAL"),
            FMOD_RESULT_FMOD_ERR_INVALID_FLOAT => bail!("FMOD_RESULT_FMOD_ERR_INVALID_FLOAT"),
            FMOD_RESULT_FMOD_ERR_INVALID_HANDLE => bail!("FMOD_RESULT_FMOD_ERR_INVALID_HANDLE"),
            FMOD_RESULT_FMOD_ERR_INVALID_PARAM => bail!("FMOD_RESULT_FMOD_ERR_INVALID_PARAM"),
            FMOD_RESULT_FMOD_ERR_INVALID_POSITION => bail!("FMOD_RESULT_FMOD_ERR_INVALID_POSITION"),
            FMOD_RESULT_FMOD_ERR_INVALID_SPEAKER => bail!("FMOD_RESULT_FMOD_ERR_INVALID_SPEAKER"),
            FMOD_RESULT_FMOD_ERR_INVALID_STRING => bail!("FMOD_RESULT_FMOD_ERR_INVALID_STRING"),
            FMOD_RESULT_FMOD_ERR_INVALID_SYNCPOINT => {
                bail!("FMOD_RESULT_FMOD_ERR_INVALID_SYNCPOINT")
            }
            FMOD_RESULT_FMOD_ERR_INVALID_THREAD => bail!("FMOD_RESULT_FMOD_ERR_INVALID_THREAD"),
            FMOD_RESULT_FMOD_ERR_INVALID_VECTOR => bail!("FMOD_RESULT_FMOD_ERR_INVALID_VECTOR"),
            FMOD_RESULT_FMOD_ERR_MAXAUDIBLE => bail!("FMOD_RESULT_FMOD_ERR_MAXAUDIBLE"),
            FMOD_RESULT_FMOD_ERR_MEMORY => bail!("FMOD_RESULT_FMOD_ERR_MEMORY"),
            FMOD_RESULT_FMOD_ERR_MEMORY_CANTPOINT => bail!("FMOD_RESULT_FMOD_ERR_MEMORY_CANTPOINT"),
            FMOD_RESULT_FMOD_ERR_NEEDS3D => bail!("FMOD_RESULT_FMOD_ERR_NEEDS3D"),
            FMOD_RESULT_FMOD_ERR_NEEDSHARDWARE => bail!("FMOD_RESULT_FMOD_ERR_NEEDSHARDWARE"),
            FMOD_RESULT_FMOD_ERR_NET_CONNECT => bail!("FMOD_RESULT_FMOD_ERR_NET_CONNECT"),
            FMOD_RESULT_FMOD_ERR_NET_SOCKET_ERROR => bail!("FMOD_RESULT_FMOD_ERR_NET_SOCKET_ERROR"),
            FMOD_RESULT_FMOD_ERR_NET_URL => bail!("FMOD_RESULT_FMOD_ERR_NET_URL"),
            FMOD_RESULT_FMOD_ERR_NET_WOULD_BLOCK => bail!("FMOD_RESULT_FMOD_ERR_NET_WOULD_BLOCK"),
            FMOD_RESULT_FMOD_ERR_NOTREADY => bail!("FMOD_RESULT_FMOD_ERR_NOTREADY"),
            FMOD_RESULT_FMOD_ERR_NOT_LOCKED => bail!("FMOD_RESULT_FMOD_ERR_NOT_LOCKED"),
            FMOD_RESULT_FMOD_ERR_OUTPUT_ALLOCATED => bail!("FMOD_RESULT_FMOD_ERR_OUTPUT_ALLOCATED"),
            FMOD_RESULT_FMOD_ERR_OUTPUT_CREATEBUFFER => {
                bail!("FMOD_RESULT_FMOD_ERR_OUTPUT_CREATEBUFFER")
            }
            FMOD_RESULT_FMOD_ERR_OUTPUT_DRIVERCALL => {
                bail!("FMOD_RESULT_FMOD_ERR_OUTPUT_DRIVERCALL")
            }
            FMOD_RESULT_FMOD_ERR_OUTPUT_FORMAT => bail!("FMOD_RESULT_FMOD_ERR_OUTPUT_FORMAT"),
            FMOD_RESULT_FMOD_ERR_OUTPUT_INIT => bail!("FMOD_RESULT_FMOD_ERR_OUTPUT_INIT"),
            FMOD_RESULT_FMOD_ERR_OUTPUT_NODRIVERS => bail!("FMOD_RESULT_FMOD_ERR_OUTPUT_NODRIVERS"),
            FMOD_RESULT_FMOD_ERR_PLUGIN => bail!("FMOD_RESULT_FMOD_ERR_PLUGIN"),
            FMOD_RESULT_FMOD_ERR_PLUGIN_MISSING => bail!("FMOD_RESULT_FMOD_ERR_PLUGIN_MISSING"),
            FMOD_RESULT_FMOD_ERR_PLUGIN_RESOURCE => bail!("FMOD_RESULT_FMOD_ERR_PLUGIN_RESOURCE"),
            FMOD_RESULT_FMOD_ERR_PLUGIN_VERSION => bail!("FMOD_RESULT_FMOD_ERR_PLUGIN_VERSION"),
            FMOD_RESULT_FMOD_ERR_RECORD => bail!("FMOD_RESULT_FMOD_ERR_RECORD"),
            FMOD_RESULT_FMOD_ERR_RECORD_DISCONNECTED => {
                bail!("FMOD_RESULT_FMOD_ERR_RECORD_DISCONNECTED")
            }
            FMOD_RESULT_FMOD_ERR_REVERB_CHANNELGROUP => {
                bail!("FMOD_RESULT_FMOD_ERR_REVERB_CHANNELGROUP")
            }
            FMOD_RESULT_FMOD_ERR_REVERB_INSTANCE => bail!("FMOD_RESULT_FMOD_ERR_REVERB_INSTANCE"),
            FMOD_RESULT_FMOD_ERR_STUDIO_NOT_LOADED => {
                bail!("FMOD_RESULT_FMOD_ERR_STUDIO_NOT_LOADED")
            }
            FMOD_RESULT_FMOD_ERR_STUDIO_UNINITIALIZED => {
                bail!("FMOD_RESULT_FMOD_ERR_STUDIO_UNINITIALIZED")
            }
            FMOD_RESULT_FMOD_ERR_SUBSOUNDS => bail!("FMOD_RESULT_FMOD_ERR_SUBSOUNDS"),
            FMOD_RESULT_FMOD_ERR_SUBSOUND_ALLOCATED => {
                bail!("FMOD_RESULT_FMOD_ERR_SUBSOUND_ALLOCATED")
            }
            FMOD_RESULT_FMOD_ERR_SUBSOUND_CANTMOVE => {
                bail!("FMOD_RESULT_FMOD_ERR_SUBSOUND_CANTMOVE")
            }
            FMOD_RESULT_FMOD_ERR_TAGNOTFOUND => bail!("FMOD_RESULT_FMOD_ERR_TAGNOTFOUND"),
            FMOD_RESULT_FMOD_ERR_TOOMANYCHANNELS => bail!("FMOD_RESULT_FMOD_ERR_TOOMANYCHANNELS"),
            FMOD_RESULT_FMOD_ERR_TOOMANYSAMPLES => bail!("FMOD_RESULT_FMOD_ERR_TOOMANYSAMPLES"),
            FMOD_RESULT_FMOD_ERR_TRUNCATED => bail!("FMOD_RESULT_FMOD_ERR_TRUNCATED"),
            FMOD_RESULT_FMOD_ERR_UNIMPLEMENTED => bail!("FMOD_RESULT_FMOD_ERR_UNIMPLEMENTED"),
            FMOD_RESULT_FMOD_ERR_UNINITIALIZED => bail!("FMOD_RESULT_FMOD_ERR_UNINITIALIZED"),
            FMOD_RESULT_FMOD_ERR_UNSUPPORTED => bail!("FMOD_RESULT_FMOD_ERR_UNSUPPORTED"),
            FMOD_RESULT_FMOD_ERR_VERSION => bail!("FMOD_RESULT_FMOD_ERR_VERSION"),
            other => unreachable!("unknown FMOD_RESULT error code: {}", other),
        }
    }
}

/// An FMOD_GUID, used to refer to event descriptions and banks. It is formatted roughly
/// like a winapi GUID. This struct has the same memory layout as the `FMOD_GUID` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct Guid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

impl Guid {
    /// Parse a Guid from a Windows-style GUID string.
    ///
    /// ```no_run
    /// # use sludge_fmod::Guid;
    /// // Note: this snippet is marked `no_run` because it's troublesome to make
    /// // doctests find the FMOD DLLs, and without them it will fail with an odd
    /// // error code.
    /// assert_eq!(
    ///     Guid::from_str("{01234567-89AB-CDEF-FEDC-BA9876543210}").unwrap(),
    ///     Guid {
    ///         data1: 0x01234567,
    ///         data2: 0x89AB,
    ///         data3: 0xCDEF,
    ///         data4: [0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10],
    ///     }
    /// );
    /// ```
    #[allow(clippy::should_implement_trait)]
    pub fn from_str<T: AsRef<str> + ?Sized>(s: &T) -> Result<Guid> {
        lazy_static! {
            static ref RE: Regex = Regex::new(
                "[{]([[:xdigit:]]{8})-([[:xdigit:]]{4})\
                    -([[:xdigit:]]{4})-([[:xdigit:]]{4})\
                    -([[:xdigit:]]{12})[}]"
            )
            .unwrap();
        }

        let caps = RE
            .captures(s.as_ref())
            .ok_or_else(|| anyhow!("couldn't parse GUID: didn't fit expected pattern"))?;
        ensure!(caps.len() == 6, "wrong number of byte groups");
        let data1 = u32::from_str_radix(caps.get(1).unwrap().as_str(), 16)?;
        let data2 = u16::from_str_radix(caps.get(2).unwrap().as_str(), 16)?;
        let data3 = u16::from_str_radix(caps.get(3).unwrap().as_str(), 16)?;

        let mut data4 = [0; 8];
        let cap4_bytes = caps.get(4).unwrap().as_str().as_bytes();
        let cap5_bytes = caps.get(5).unwrap().as_str().as_bytes();

        let seg1 = cap4_bytes.chunks(2).map(str::from_utf8);
        let seg2 = cap5_bytes.chunks(2).map(str::from_utf8);

        for (i, chunk) in seg1.chain(seg2).enumerate() {
            data4[i] = u8::from_str_radix(chunk?, 16)?;
        }

        Ok(Guid {
            data1,
            data2,
            data3,
            data4,
        })
    }
}

bitflags::bitflags! {
    /// Options for initializing the FMOD Studio System object.
    pub struct FmodStudioInitFlags: u32 {
        /// No special options, the default.
        const NORMAL                = FMOD_STUDIO_INIT_NORMAL;
        /// Enable FMOD's live update functionality.
        const LIVEUPDATE            = FMOD_STUDIO_INIT_LIVEUPDATE;
        const ALLOW_MISSING_PLUGINS = FMOD_STUDIO_INIT_ALLOW_MISSING_PLUGINS;
        /// Disable asynchronous processing/multithreading and instead perform all FMOD
        /// updates/processing on the main thread when `Fmod::update` is called. This
        /// can be dangerous as it will cause FMOD Studio to assume that all FMOD
        /// API calls will come from a single thread! As such we currently will
        /// panic if this option is passed in.
        const SYNCHRONOUS_UPDATE    = FMOD_STUDIO_INIT_SYNCHRONOUS_UPDATE;
        /// Defer callbacks until `Fmod::update`. Useful for ensuring your callbacks
        /// fire on the main thread and non-concurrently to whatever they modify.
        const DEFERRED_CALLBACKS    = FMOD_STUDIO_INIT_DEFERRED_CALLBACKS;
        /// Perform resource loading from `Fmod::update` rather than asynchronously.
        const LOAD_FROM_UPDATE      = FMOD_STUDIO_INIT_LOAD_FROM_UPDATE;
        /// Enable detailed memory usage statistics. This option increases the memory
        /// footprint of FMOD significantly and will impact performance.
        const MEMORY_TRACKING       = FMOD_STUDIO_INIT_MEMORY_TRACKING;
    }
}

bitflags::bitflags! {
    pub struct FmodCoreInitFlags: u32 {
        const NORMAL                 = FMOD_INIT_NORMAL;
        const STREAM_FROM_UPDATE     = FMOD_INIT_STREAM_FROM_UPDATE;
        const MIX_FROM_UPDATE        = FMOD_INIT_MIX_FROM_UPDATE;
        const _3D_RIGHTHANDED        = FMOD_INIT_3D_RIGHTHANDED;
        const CHANNEL_LOWPASS        = FMOD_INIT_CHANNEL_LOWPASS;
        const CHANNEL_DISTANCEFILTER = FMOD_INIT_CHANNEL_DISTANCEFILTER;
        const PROFILE_ENABLE         = FMOD_INIT_PROFILE_ENABLE;
        const VOL0_BECOMES_VIRTUAL   = FMOD_INIT_VOL0_BECOMES_VIRTUAL;
        const GEOMETRY_USECLOSEST    = FMOD_INIT_GEOMETRY_USECLOSEST;
        const PREFER_DOLBY_DOWNMIX   = FMOD_INIT_PREFER_DOLBY_DOWNMIX;
        const THREAD_UNSAFE          = FMOD_INIT_THREAD_UNSAFE;
        const PROFILE_METER_ALL      = FMOD_INIT_PROFILE_METER_ALL;
        const MEMORY_TRACKING        = FMOD_INIT_MEMORY_TRACKING;
    }
}

/// A builder struct for initializing the FMOD Studio System. At current we don't
/// really have any options to set here in between `create` and `initialize` but
/// they'll be implemented eventually.
pub struct FmodSystemBuilder {
    system: *mut FMOD_STUDIO_SYSTEM,
}

impl FmodSystemBuilder {
    /// Initialize the builder's internal `FMOD_STUDIO_SYSTEM` object.
    pub fn create() -> Result<Self> {
        let mut system = ptr::null_mut();

        unsafe {
            FMOD_Studio_System_Create(&mut system, FMOD_VERSION).check_err()?;
        }

        Ok(Self { system })
    }

    /// Initialize the builder's FMOD studio system object, finishing the building
    /// process.
    pub fn initialize(
        self,
        max_channels: u32,
        studio_flags: FmodStudioInitFlags,
        core_flags: FmodCoreInitFlags,
    ) -> Result<Fmod> {
        ensure!(
            !studio_flags.contains(FmodStudioInitFlags::SYNCHRONOUS_UPDATE)
                && !core_flags.contains(FmodCoreInitFlags::THREAD_UNSAFE),
            "initialization flags contain options which disable thread safety \
             and are not currently supported!"
        );

        unsafe {
            FMOD_Studio_System_Initialize(
                self.system,
                max_channels as i32,
                studio_flags.bits,
                core_flags.bits,
                ptr::null_mut(),
            )
            .check_err()?;
        }

        let (cq_send, cq_recv) = std::sync::mpsc::channel();
        let fmod = Fmod {
            ptr: self.system,
            callbacks: Mutex::new(Arena::new()),
            cleanup: Arc::new(RwLock::new(AtomicBitSet::new())),
            cq_recv,
            cq_send,
        };

        Ok(fmod)
    }
}

/// This is our main FMOD context type, representing the studio system object.
///
/// This type will automatically destroy the FMOD Core/Studio API objects when it is dropped.
#[derive(Debug)]
pub struct Fmod {
    pub(crate) ptr: *mut FMOD_STUDIO_SYSTEM,

    callbacks: Mutex<Arena<LuaRegistryKey>>,
    cleanup: Arc<RwLock<AtomicBitSet>>,

    pub(crate) cq_recv: Receiver<(Index, EventInstance, EventCallbackInfo)>,
    pub(crate) cq_send: Sender<(Index, EventInstance, EventCallbackInfo)>,
}

// FMOD Studio API is thread safe by default, and we panic if we see something which
// would cause otherwise in `Fmod::new()`. So this is okay.
unsafe impl Send for Fmod {}
unsafe impl Sync for Fmod {}

impl Fmod {
    /// This function should be called in your game's update loop.
    ///
    /// Ideally, you should call `update` *after* your game might make FMOD API
    /// calls, so that any calls are taken care of as quickly as possible.
    /// However, you should note that any callbacks which are set through the Lua
    /// API into things like FMOD event instances looking for beat callbacks or
    /// timeline marker callbacks will only be fired when `flush_callbacks` is
    /// called after the event occurs. As such, it's recommended that you call
    /// `flush_callbacks` immediately before your game update, in order to receive
    /// new callback events, and then `update` afterwards in order to flush any
    /// newly recorded commands to FMOD's asynchronous processing system.
    pub fn update(&self) -> Result<()> {
        unsafe {
            FMOD_Studio_System_Update(self.ptr).check_err()?;
        }
        Ok(())
    }

    /// If callbacks are registered through the Lua system, then their execution
    /// is deferred by sending their parameters into a queue in the `Fmod` object
    /// and then flushing the queue with this method and calling all the relevant
    /// Lua closures.
    pub fn flush_callbacks(&self, lua: &Lua) -> Result<()> {
        {
            let callbacks = &mut self.callbacks.lock().unwrap();
            for (_, key) in self
                .cleanup
                .borrow_mut()
                .drain()
                .filter_map(|i| callbacks.remove_by_slot(i))
            {
                lua.remove_registry_value(key)?;
            }
        }

        for (index, event_instance, event_info) in self.cq_recv.try_iter() {
            let cb = {
                let callbacks = &self.callbacks.lock().unwrap();
                lua.registry_value::<LuaFunction>(&callbacks[index])?
            };

            use EventCallbackInfo::*;
            match event_info {
                Created => cb.call((event_instance, "created"))?,
                Destroyed => cb.call((event_instance, "destroyed"))?,
                Starting => cb.call((event_instance, "starting"))?,
                Started => cb.call((event_instance, "started"))?,
                Restarted => cb.call((event_instance, "restarted"))?,
                Stopped => cb.call((event_instance, "stopped"))?,
                StartFailed => cb.call((event_instance, "start_failed"))?,
                //CreateProgrammerSound(&'a Sound) => CreateProgrammerSound(&'a Sound),
                //DestroyProgrammerSound(&'a Sound) => DestroyProgrammerSound(&'a Sound),
                //PluginCreated(PluginInstanceProperties) => PluginCreated(PluginInstanceProperties),
                //PluginDestroyed(PluginInstanceProperties) => PluginDestroyed(PluginInstanceProperties),
                TimelineMarker(marker) => {
                    cb.call((event_instance, "timeline_marker", marker.to_lua(lua)?))?
                }
                TimelineBeat(beat) => {
                    cb.call((event_instance, "timeline_beat", beat.to_lua(lua)?))?
                }
                //SoundPlayed(&'a Sound) => SoundPlayed(&'a Sound),
                //SoundStopped(&'a Sound) => SoundStopped(&'a Sound),
                RealToVirtual => cb.call((event_instance, "real_to_virtual"))?,
                VirtualToReal => cb.call((event_instance, "virtual_to_real"))?,
                StartEventCommand(other_event_instance) => {
                    cb.call((event_instance, "start_event_command", other_event_instance))?
                }
            }
        }

        Ok(())
    }

    /// Load a bank file from a path, relative to your current directory. Banks will not be
    /// unloaded by dropping the `Bank` object, and must be manually released if desired either
    /// through `Bank::unload` or `Fmod::unloadAll`.
    pub fn load_bank_file<T: AsRef<[u8]>>(
        &self,
        filename: T,
        flags: LoadBankFlags,
    ) -> Result<Bank> {
        let c_string = CString::new(filename.as_ref())?;
        let mut ptr = ptr::null_mut();
        unsafe {
            FMOD_Studio_System_LoadBankFile(self.ptr, c_string.as_ptr(), flags.bits(), &mut ptr)
                .check_err()?;
            Ok(Bank::from_ptr(ptr))
        }
    }

    /// Unload all currently loaded banks.
    pub fn unload_all(&self) -> Result<()> {
        let banks = self.get_bank_list()?;
        for event in banks
            .into_iter()
            .flat_map(|bank| bank.get_event_list())
            .flatten()
        {
            event.unset_callback()?;
        }

        unsafe {
            FMOD_Studio_System_UnloadAll(self.ptr).check_err()?;
        }
        Ok(())
    }

    /// Retrieve a loaded bank by its path.
    pub fn get_bank<T: AsRef<[u8]>>(&self, filename: T) -> Result<Bank> {
        let c_string = CString::new(filename.as_ref())?;
        let mut ptr = ptr::null_mut();
        unsafe {
            FMOD_Studio_System_GetBank(self.ptr, c_string.as_ptr(), &mut ptr).check_err()?;
            Ok(Bank::from_ptr(ptr))
        }
    }

    /// Retrieve a loaded bank by its GUID.
    pub fn get_bank_by_id(&self, guid: &Guid) -> Result<Bank> {
        let mut ptr = ptr::null_mut();
        unsafe {
            FMOD_Studio_System_GetBankByID(self.ptr, guid as *const _ as *mut _, &mut ptr)
                .check_err()?;
            Ok(Bank::from_ptr(ptr))
        }
    }

    /// Returns the number of currently loaded banks.
    pub fn get_bank_count(&self) -> Result<u32> {
        let mut out = 0;
        unsafe {
            FMOD_Studio_System_GetBankCount(self.ptr, &mut out).check_err()?;
        }
        Ok(out as u32)
    }

    /// Retrieve all currently loaded banks from the Studio System object and return them in
    /// a `Vec`, in unspecified order.
    pub fn get_bank_list(&self) -> Result<Vec<Bank>> {
        unsafe {
            let mut banks = vec![Bank::from_ptr(ptr::null_mut()); self.get_bank_count()? as usize];
            let mut count_out = 0;
            FMOD_Studio_System_GetBankList(
                self.ptr,
                banks.as_mut_ptr() as *mut *mut FMOD_STUDIO_BANK,
                banks.len() as i32,
                &mut count_out,
            )
            .check_err()?;
            banks.truncate(count_out as usize);
            Ok(banks)
        }
    }

    /// Get a loaded event by its path or ID string (GUID in its string format; see [`Guid`][Guid]).
    pub fn get_event<T: AsRef<[u8]> + ?Sized>(&self, path: &T) -> Result<EventDescription> {
        let c_string = CString::new(path.as_ref())?;
        let mut ptr = ptr::null_mut();
        unsafe {
            FMOD_Studio_System_GetEvent(self.ptr, c_string.as_ptr(), &mut ptr).check_err()?;
            EventDescription::from_ptr(ptr)
        }
    }

    /// Get a loaded event by its [GUID][Guid].
    pub fn get_event_by_id(&self, guid: &Guid) -> Result<EventDescription> {
        let mut ptr = ptr::null_mut();
        unsafe {
            FMOD_Studio_System_GetEventByID(self.ptr, guid as *const _ as *mut _, &mut ptr)
                .check_err()?;
            EventDescription::from_ptr(ptr)
        }
    }

    pub(crate) fn insert_callback(&self, callback: LuaRegistryKey) -> CallbackDropGuard {
        CallbackDropGuard {
            cleanup: self.cleanup.clone(),
            index: self.callbacks.lock().unwrap().insert(callback),
        }
    }
}

impl Drop for Fmod {
    fn drop(&mut self) {
        unsafe {
            FMOD_Studio_System_Release(self.ptr)
                .check_err()
                .expect("error dropping FMOD system");
        }
    }
}

impl LuaUserData for Fmod {}

impl LuaResource for Fmod {
    const REGISTRY_KEY: &'static str = "HV_FMOD";
}

#[derive(Debug)]
pub(crate) struct CallbackDropGuard {
    cleanup: Arc<RwLock<AtomicBitSet>>,
    index: Index,
}

impl Drop for CallbackDropGuard {
    fn drop(&mut self) {
        self.cleanup.read().unwrap().add_atomic(self.index.slot());
    }
}

struct HvFmodPlugin;

impl Plugin for HvFmodPlugin {
    fn name(&self) -> &'static str {
        "fmod"
    }

    fn open<'lua>(&self, lua: &'lua Lua, engine: &Engine) -> Result<LuaTable<'lua>> {
        let fmod_resource = engine.insert(FmodSystemBuilder::create()?.initialize(
            1024,
            FmodStudioInitFlags::NORMAL,
            FmodCoreInitFlags::NORMAL,
        )?);
        lua.register(fmod_resource.clone())?;

        let fmod = fmod_resource.clone();
        let load_bank_file = lua.create_function(
            move |_lua, (filename, flags): (LuaString, Option<LoadBankFlags>)| {
                let bank = fmod
                    .borrow()
                    .load_bank_file(filename.as_bytes(), flags.unwrap_or(LoadBankFlags::NORMAL))
                    .to_lua_err()?;
                Ok(bank)
            },
        )?;

        let fmod = fmod_resource;
        let get_event = lua.create_function(move |_lua, path: LuaString| {
            let event = fmod.borrow().get_event(path.as_bytes()).to_lua_err()?;
            Ok(event)
        })?;

        let load_bank_flags = lua.create_table_from(vec![
            ("NORMAL", LoadBankFlags::NORMAL),
            ("NONBLOCKING", LoadBankFlags::NONBLOCKING),
            ("DECOMPRESS_SAMPLES", LoadBankFlags::DECOMPRESS_SAMPLES),
            ("UNENCRYPTED", LoadBankFlags::UNENCRYPTED),
        ])?;

        let event_callback_mask = lua.create_table_from(vec![
            ("CREATED", EventCallbackMask::CREATED),
            ("DESTROYED", EventCallbackMask::DESTROYED),
            ("STARTING", EventCallbackMask::STARTING),
            ("STARTED", EventCallbackMask::STARTED),
            ("RESTARTED", EventCallbackMask::RESTARTED),
            ("STOPPED", EventCallbackMask::STOPPED),
            ("START_FAILED", EventCallbackMask::START_FAILED),
            (
                "CREATE_PROGRAMMER_SOUND",
                EventCallbackMask::CREATE_PROGRAMMER_SOUND,
            ),
            (
                "DESTROY_PROGRAMMER_SOUND",
                EventCallbackMask::DESTROY_PROGRAMMER_SOUND,
            ),
            ("PLUGIN_CREATED", EventCallbackMask::PLUGIN_CREATED),
            ("PLUGIN_DESTROYED", EventCallbackMask::PLUGIN_DESTROYED),
            ("TIMELINE_MARKER", EventCallbackMask::TIMELINE_MARKER),
            ("TIMELINE_BEAT", EventCallbackMask::TIMELINE_BEAT),
            ("SOUND_PLAYED", EventCallbackMask::SOUND_PLAYED),
            ("SOUND_STOPPED", EventCallbackMask::SOUND_STOPPED),
            ("REAL_TO_VIRTUAL", EventCallbackMask::REAL_TO_VIRTUAL),
            ("VIRTUAL_TO_REAL", EventCallbackMask::VIRTUAL_TO_REAL),
            (
                "START_EVENT_COMMAND",
                EventCallbackMask::START_EVENT_COMMAND,
            ),
            ("ALL", EventCallbackMask::ALL),
        ])?;

        Ok(lua
            .load(mlua::chunk! {
                {
                    load_bank_file = $load_bank_file,
                    get_event = $get_event,

                    EventCallbackMask = $event_callback_mask,
                    LoadBankFlags = $load_bank_flags,
                }
            })
            .eval()?)
    }
}

hv_core::plugin!(HvFmodPlugin);

pub fn link_me() {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn guid() {
        unsafe {
            assert_eq!(mem::size_of::<FMOD_GUID>(), mem::size_of::<Guid>());

            let fmod_guid = FMOD_GUID {
                Data1: 0x01234567,
                Data2: 0x89AB,
                Data3: 0xCDEF,
                Data4: [0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10],
            };

            let mut rust_guid = Guid {
                data1: 0,
                data2: 0,
                data3: 0,
                data4: [0; 8],
            };

            *((&mut rust_guid) as *mut Guid as *mut FMOD_GUID) = fmod_guid;

            assert_eq!(fmod_guid.Data1, rust_guid.data1);
            assert_eq!(fmod_guid.Data2, rust_guid.data2);
            assert_eq!(fmod_guid.Data3, rust_guid.data3);
            assert_eq!(fmod_guid.Data4, rust_guid.data4);

            assert_eq!(
                Guid::from_str("{01234567-89AB-CDEF-FEDC-BA9876543210}").unwrap(),
                Guid {
                    data1: 0x01234567,
                    data2: 0x89AB,
                    data3: 0xCDEF,
                    data4: [0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10],
                }
            );
        }
    }
}
