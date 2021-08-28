use hv_core::{engine::WeakResourceCache, prelude::*};

use crate::graphics::{DrawableMut, GraphicsLock, GraphicsLockExt, Instance};

#[macro_export]
macro_rules! add_field {
    ($fields:ident, $t:ident.$n:ident => $f:expr) => {
        $crate::lua::add_field_getter($fields, stringify!($n), |$t| $f);
        $crate::lua::add_field_setter($fields, stringify!($n), |$t, u| $f = u);
    };
}

#[macro_export]
macro_rules! add_getter {
    ($fields:ident, $t:ident.$n:ident => $f:expr) => {
        $crate::lua::add_field_getter($fields, stringify!($n), |$t| $f);
    };
}

#[macro_export]
macro_rules! add_setter {
    ($fields:ident, $t:ident.$n:ident = $v:ident $(: $vt:ty)? => $f:expr) => {
        $crate::lua::add_field_setter($fields, stringify!($n), |$t, $v $(: $vt)?| $f);
    };
}

pub fn simple<'lua, M, T, A, R, F, S>(methods: &mut M, name: &S, f: F)
where
    M: LuaUserDataMethods<'lua, T>,
    T: Send + LuaUserData + 'static,
    A: FromLuaMulti<'lua> + Send,
    R: ToLuaMulti<'lua> + Send,
    F: Fn(&T, A) -> R + Send + Sync + 'static,
    S: AsRef<[u8]> + ?Sized,
{
    methods.add_method(name, move |_, this, args| Ok(f(this, args)));
}

pub fn simple_mut<'lua, M, T, A, R, F, S>(methods: &mut M, name: &S, mut f: F)
where
    M: LuaUserDataMethods<'lua, T>,
    T: Send + LuaUserData + 'static,
    A: FromLuaMulti<'lua> + Send,
    R: ToLuaMulti<'lua> + Send,
    F: FnMut(&mut T, A) -> R + Send + Sync + 'static,
    S: AsRef<[u8]> + ?Sized,
{
    methods.add_method_mut(name, move |_, this, args| Ok(f(this, args)));
}

pub fn add_clone_methods<'lua, M, T>(methods: &mut M)
where
    M: LuaUserDataMethods<'lua, T>,
    T: Clone + Send + LuaUserData + 'static,
{
    methods.add_method("clone", move |_lua, this, ()| Ok(this.clone()));
    methods.add_method_mut("clone_from", move |_lua, this, rhs: LuaAnyUserData| {
        this.clone_from(&*rhs.borrow::<T>()?);
        Ok(())
    });
}

pub fn add_field_getter<'lua, M, T, U, F, S>(fields: &mut M, name: &S, f: F)
where
    M: LuaUserDataFields<'lua, T>,
    T: Send + LuaUserData + 'static,
    U: ToLua<'lua> + Send + 'static,
    F: Fn(&T) -> U + Send + Sync + 'static,
    S: AsRef<[u8]> + ?Sized,
{
    fields.add_field_method_get(name, move |_, this| Ok(f(this)));
}

pub fn add_field_setter<'lua, M, T, U, F, S>(fields: &mut M, name: &S, f: F)
where
    M: LuaUserDataFields<'lua, T>,
    T: Send + LuaUserData + 'static,
    U: FromLua<'lua>,
    F: Fn(&mut T, U) + Send + Sync + 'static,
    S: AsRef<[u8]> + ?Sized,
{
    fields.add_field_method_set(name, move |_, this, u: U| {
        f(this, u);
        Ok(())
    });
}

// pub fn lh_binop<'lua, M, L, R, F, S>(methods: &mut M, name: &S, f: F)
// where
//     M: LuaUserDataMethods<'lua, L>,
//     L: Copy + Send + LuaUserData + 'static,
//     R: Copy + Send + LuaUserData + 'static,
//     F: Fn(L, R) -> L + Send + Sync + 'static,
//     S: AsRef<[u8]> + ?Sized,
// {
//     methods.add_function(
//         name,
//         move |_, (out_ud, lhs_ud, rhs_ud): (LuaAnyUserData, LuaAnyUserData, LuaAnyUserData)| {
//             let lhs = *lhs_ud.borrow::<L>()?;
//             let rhs = *rhs_ud.borrow::<R>()?;
//             let tmp = f(lhs, rhs);
//             *out_ud.borrow_mut()? = tmp;
//             Ok(out_ud)
//         },
//     );
// }

pub fn add_drawable_methods<'lua, M, T>(methods: &mut M)
where
    M: LuaUserDataMethods<'lua, T>,
    T: DrawableMut + LuaUserData,
{
    let mut weak_gfx_cache = WeakResourceCache::<GraphicsLock>::new();
    methods.add_method_mut("draw", move |lua, this, maybe_params: Option<Instance>| {
        let gfx_lock = weak_gfx_cache.get(|| lua.resource::<GraphicsLock>())?;
        this.draw_mut(&mut gfx_lock.lock(), maybe_params.unwrap_or_default());
        Ok(())
    });
}
