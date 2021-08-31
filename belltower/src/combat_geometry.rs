use hv_core::{
    components::DynamicComponentConstructor,
    engine::Engine,
    plugins::Plugin,
    prelude::*,
    spaces::{Object, SpaceCache},
    util::RwLockExt,
};
use hv_friends::{
    graphics::{Color, DrawMode, MeshBuilder},
    math::*,
    parry2d::shape::Cuboid,
};
use serde::*;

use crate::box_geometry::{BoxCollider, BoxGeometry};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BoxType {
    Hurt,
    Hit,
    Parry,
}

impl<'lua> FromLua<'lua> for BoxType {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        lua.from_value(lua_value)
    }
}

pub struct CombatGeometry {
    geometry: BoxGeometry<BoxType>,
}

impl CombatGeometry {
    pub fn new() -> Self {
        Self {
            geometry: BoxGeometry::new(),
        }
    }

    pub fn geometry(&self) -> &BoxGeometry<BoxType> {
        &self.geometry
    }

    pub fn geometry_mut(&mut self) -> &mut BoxGeometry<BoxType> {
        &mut self.geometry
    }

    pub fn add(&mut self, cuboid: Cuboid, tx: Isometry2<f32>, properties: BoxType) -> &mut Self {
        self.geometry.insert(BoxCollider {
            cuboid,
            tx,
            properties,
        });
        self
    }

    pub fn hitbox(&mut self, cuboid: Cuboid, tx: Isometry2<f32>) -> &mut Self {
        self.add(cuboid, tx, BoxType::Hit)
    }

    pub fn hurtbox(&mut self, cuboid: Cuboid, tx: Isometry2<f32>) -> &mut Self {
        self.add(cuboid, tx, BoxType::Hurt)
    }

    pub fn parrybox(&mut self, cuboid: Cuboid, tx: Isometry2<f32>) -> &mut Self {
        self.add(cuboid, tx, BoxType::Parry)
    }

    pub fn clear(&mut self) {
        self.geometry.clear();
    }

    pub fn append_debug_polygons_to_mesh(&self, mesh_builder: &mut MeshBuilder) -> Result<()> {
        for (_, elem) in self.geometry.iter() {
            let points = elem.to_points();
            let color = match elem.properties {
                BoxType::Hit => Color::CYAN,
                BoxType::Hurt => Color::RED,
                BoxType::Parry => Color::YELLOW,
            };

            mesh_builder.polygon(DrawMode::fill(), &points, color)?;
        }

        Ok(())
    }
}

impl BoxCollider<BoxType> {
    pub fn hurt(cuboid: Cuboid, tx: Isometry2<f32>) -> Self {
        BoxCollider {
            cuboid,
            tx,
            properties: BoxType::Hurt,
        }
    }

    pub fn parry(cuboid: Cuboid, tx: Isometry2<f32>) -> Self {
        BoxCollider {
            cuboid,
            tx,
            properties: BoxType::Parry,
        }
    }
}

struct CombatGeometryComponentPlugin;

impl Plugin for CombatGeometryComponentPlugin {
    fn name(&self) -> &'static str {
        "game.CombatGeometry"
    }

    fn open<'lua>(&self, lua: &'lua Lua, engine: &Engine) -> Result<LuaTable<'lua>> {
        let new_cg = lua.create_function(|_, ()| {
            Ok(DynamicComponentConstructor::new(|_: &Lua, _| {
                Ok(CombatGeometry::new())
            }))
        })?;

        let mut space_cache = SpaceCache::new(engine);
        let cg_clear = lua.create_function_mut(move |_, object: Object| {
            let space = space_cache.get_space(object.space());
            space
                .borrow()
                .get_mut::<CombatGeometry>(object)
                .to_lua_err()?
                .clear();
            Ok(())
        })?;

        let mut space_cache = SpaceCache::new(engine);
        let cg_add = lua.create_function_mut(move |_, (object, table): (Object, LuaTable)| {
            let space = space_cache.get_space(object.space());

            let cuboid = Cuboid::new(Vector2::new(
                table.get("half_width")?,
                table.get("half_height")?,
            ));
            let tx = Isometry2::new(
                Vector2::new(table.get("x")?, table.get("y")?),
                table.get("angle")?,
            );
            let properties = table.get("ty")?;

            space
                .borrow()
                .get_mut::<CombatGeometry>(object)
                .to_lua_err()?
                .add(cuboid, tx, properties);

            Ok(())
        })?;

        Ok(lua
            .load(mlua::chunk! {
                local CombatGeometry = {
                    clear = $cg_clear,
                    add = $cg_add
                }

                return setmetatable(CombatGeometry, { __call = $new_cg })
            })
            .eval()?)
    }
}

hv_core::component!(CombatGeometryComponentPlugin);
