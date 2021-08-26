use crate::{event::EventDescription, CheckError};
use {hv_core::prelude::*, hv_fmod_sys::*, std::ptr};

bitflags::bitflags! {
    pub struct LoadBankFlags: u32 {
        const NORMAL = FMOD_STUDIO_LOAD_BANK_NORMAL;
        const NONBLOCKING = FMOD_STUDIO_LOAD_BANK_NONBLOCKING;
        const DECOMPRESS_SAMPLES = FMOD_STUDIO_LOAD_BANK_DECOMPRESS_SAMPLES;
        const UNENCRYPTED = FMOD_STUDIO_LOAD_BANK_UNENCRYPTED;
    }
}

impl<'lua> ToLua<'lua> for LoadBankFlags {
    fn to_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        self.bits().to_lua(lua)
    }
}

impl<'lua> FromLua<'lua> for LoadBankFlags {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Self::from_bits(u32::from_lua(lua_value, lua)?)
            .ok_or_else(|| anyhow!("invalid bank load flags"))
            .to_lua_err()
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Bank {
    pub(crate) ptr: *mut FMOD_STUDIO_BANK,
}

unsafe impl Send for Bank {}
unsafe impl Sync for Bank {}

impl Bank {
    pub(crate) unsafe fn from_ptr(ptr: *mut FMOD_STUDIO_BANK) -> Self {
        Self { ptr }
    }

    pub fn is_valid(&self) -> bool {
        unsafe { FMOD_Studio_Bank_IsValid(self.ptr) != 0 }
    }

    pub fn load_sample_data(&self) -> Result<()> {
        unsafe {
            FMOD_Studio_Bank_LoadSampleData(self.ptr).check_err()?;
        }
        Ok(())
    }

    pub fn unload_sample_data(&self) -> Result<()> {
        unsafe {
            FMOD_Studio_Bank_UnloadSampleData(self.ptr).check_err()?;
        }
        Ok(())
    }

    pub fn get_event_count(&self) -> Result<u32> {
        let mut count = 0;
        unsafe {
            FMOD_Studio_Bank_GetEventCount(self.ptr, &mut count).check_err()?;
        }
        Ok(count as u32)
    }

    pub fn get_event_list(&self) -> Result<Vec<EventDescription>> {
        let mut events;
        let mut count = 0;
        unsafe {
            let null_desc = EventDescription {
                ptr: ptr::null_mut(),
            };
            events = vec![null_desc; self.get_event_count()? as usize];
            FMOD_Studio_Bank_GetEventList(
                self.ptr,
                events.as_mut_ptr() as *mut *mut FMOD_STUDIO_EVENTDESCRIPTION,
                events.len() as i32,
                &mut count,
            )
            .check_err()?;

            // Properly initialize them while reusing the memory.
            for event in &mut events {
                *event = EventDescription::from_ptr(event.ptr)?;
            }
        }

        events.truncate(count as usize);
        Ok(events)
    }

    pub fn unload(&self) -> Result<()> {
        for event in self.get_event_list()? {
            event.unset_callback()?;
        }

        unsafe {
            FMOD_Studio_Bank_Unload(self.ptr).check_err()?;
        }
        Ok(())
    }
}

impl LuaUserData for Bank {
    fn add_methods<'lua, T: LuaUserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_method("is_valid", |_lua, this, ()| Ok(this.is_valid()));

        methods.add_method("load_sample_data", |_lua, this, ()| {
            this.load_sample_data().to_lua_err()?;
            Ok(())
        });

        methods.add_method("unload_sample_data", |_lua, this, ()| {
            this.unload_sample_data().to_lua_err()?;
            Ok(())
        });
    }
}

// inventory::submit! {
//     Module::parse("fmod.LoadBankFlags", load)
// }
