use aseprite::SpritesheetData;
use hv_core::{
    engine::{Engine, EngineRef, LuaResource},
    mq,
    prelude::*,
    swappable_cache::{CacheRef, Guard, Handle, Loader, SwappableCache},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io::Read, mem, ops, path::Path};
use thunderdome::{Arena, Index};

use crate::{
    graphics::{CachedTexture, Drawable, DrawableMut, Graphics, Instance, InstanceProperties},
    math::*,
};

/// An image plus an instance parameter. Useful for drawing a single piece of a spritesheet, for
/// example, without dealing with the extra machinery required for a spritebatch. [`Sprite`] will
/// also, unlike a type from the [`texture`] family, scale itself according to the provided UVs so
/// that by default it is always at the correct pixel scale; if you use UVs from `(0, 0)` to `(0.5,
/// 0.5)` on a 16x16 texture, for example, then the [`Sprite`] will render an 8x8 chunk of it.
///
/// [`texture`]: crate::graphics::texture
#[derive(Debug, Clone)]
pub struct Sprite {
    /// The instance parameters for this.
    pub params: Instance,
    /// The sprite's texture.
    pub texture: CachedTexture,
}

impl Sprite {
    /// Create a new sprite from a texture type and an instance.
    pub fn new(texture: impl Into<CachedTexture>, params: Instance) -> Self {
        Self {
            params,
            texture: texture.into(),
        }
    }
}

impl DrawableMut for Sprite {
    fn draw_mut(&mut self, ctx: &mut Graphics, instance: Instance) {
        let params = Instance {
            tx: instance.tx * self.params.tx,
            ..self.params
        };
        self.texture
            .draw_mut(ctx, params.scale2(params.src.extents()));
    }
}

/// FIXME(sleffy): same issue as the SpriteBatch implementation, ignoring
/// the passed-in src/color params
impl Drawable for Sprite {
    fn draw(&self, ctx: &mut Graphics, instance: Instance) {
        let params = Instance {
            tx: instance.tx * self.params.tx,
            ..self.params
        };
        self.texture
            .get()
            .draw(ctx, params.scale2(params.src.extents()));
    }
}

impl LuaUserData for Sprite {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        crate::lua::add_drawable_methods(methods);
    }
}

/// Represents the index of a "sprite" within a [`SpriteBatch`].
///
/// Useful for uniquely identifying a "live" sprite in a batch, if not using the slot API instead.
/// Internally this is a generational index (a slot plus a generation counter), so unlike a slot,
/// this won't end up referring to a "resurrected" index in the [`SpriteBatch`]. This isn't often an
/// issue, but for more information, please look into the "ABA problem".
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SpriteId(Index);

impl SpriteId {
    /// Get the slot of this `SpriteId`.
    pub fn slot(self) -> u32 {
        self.0.slot()
    }
}

impl<'lua> ToLua<'lua> for SpriteId {
    fn to_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        LuaLightUserData(self.0.to_bits() as *mut _).to_lua(lua)
    }
}

impl<'lua> FromLua<'lua> for SpriteId {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        LuaLightUserData::from_lua(lua_value, lua).map(|lud| Self(Index::from_bits(lud.0 as u64)))
    }
}

/// An iterator offering immutable access to all of the sprite instances in a batch.
pub struct SpriteBatchIter<'a> {
    iter: thunderdome::Iter<'a, Instance>,
}

impl<'a> Iterator for SpriteBatchIter<'a> {
    type Item = (SpriteId, &'a Instance);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(i, v)| (SpriteId(i), v))
    }
}

/// An iterator offering mutable access to all of the sprite instances in a batch.
pub struct SpriteBatchIterMut<'a> {
    iter: thunderdome::IterMut<'a, Instance>,
}

impl<'a> Iterator for SpriteBatchIterMut<'a> {
    type Item = (SpriteId, &'a mut Instance);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(i, v)| (SpriteId(i), v))
    }
}

/// A collection of [`Instance`]s with an associated texture, rendered efficiently as an instanced
/// batch.
///
/// If you have a lot of [`Sprite`]s using the same texture, this is a much more efficient way to
/// render them. Way more efficient.
#[derive(Debug)]
pub struct SpriteBatch {
    sprites: Arena<Instance>,
    // Used to store the result of converting InstanceParams to InstanceProperties
    instances: Vec<InstanceProperties>,
    // Capacity is used to store the length of the buffers inside of mq::Bindings
    capacity: usize,
    bindings: mq::Bindings,
    dirty: bool,
    texture: CachedTexture,
}

impl ops::Index<SpriteId> for SpriteBatch {
    type Output = Instance;

    #[inline]
    fn index(&self, index: SpriteId) -> &Self::Output {
        &self.sprites[index.0]
    }
}

impl ops::IndexMut<SpriteId> for SpriteBatch {
    #[inline]
    fn index_mut(&mut self, index: SpriteId) -> &mut Self::Output {
        self.dirty = true;
        &mut self.sprites[index.0]
    }
}

impl SpriteBatch {
    /// Create a new spritebatch for the given texture.
    pub fn new<T>(ctx: &mut Graphics, texture: T) -> Self
    where
        T: Into<CachedTexture>,
    {
        const DEFAULT_SPRITEBATCH_CAPACITY: usize = 64;
        Self::with_capacity(ctx, texture, DEFAULT_SPRITEBATCH_CAPACITY)
    }

    /// Create a new spritebatch for the given texture and with the given initial capacity.
    pub fn with_capacity<T>(ctx: &mut Graphics, texture: T, capacity: usize) -> Self
    where
        T: Into<CachedTexture>,
    {
        let mut texture = texture.into();

        let instances = mq::Buffer::stream(
            &mut ctx.mq,
            mq::BufferType::VertexBuffer,
            capacity * mem::size_of::<InstanceProperties>(),
        );

        let bindings = mq::Bindings {
            vertex_buffers: vec![ctx.state.quad_bindings.vertex_buffers[0], instances],
            index_buffer: ctx.state.quad_bindings.index_buffer,
            images: vec![texture.get_cached().handle],
        };

        Self {
            sprites: Arena::new(),
            instances: Vec::new(),
            capacity,
            bindings,
            dirty: true,
            texture,
        }
    }

    /// Insert a single sprite into the batch as an instance parameter, and get a unique identifier
    /// referring to it.
    #[inline]
    pub fn insert(&mut self, param: Instance) -> SpriteId {
        self.dirty = true;
        SpriteId(self.sprites.insert(param))
    }

    /// Remove a sprite from the batch, by its ID.
    #[inline]
    pub fn remove(&mut self, index: SpriteId) -> Option<Instance> {
        self.dirty = true;
        self.sprites.remove(index.0)
    }

    /// Remove a sprite from the batch, using only its slot and ignoring the generational component
    /// of its ID. Returns the corresponding sprite ID that was removed if successful.
    #[inline]
    pub fn remove_by_slot(&mut self, slot: u32) -> Option<(SpriteId, Instance)> {
        self.dirty = true;
        self.sprites
            .remove_by_slot(slot)
            .map(|(index, instance)| (SpriteId(index), instance))
    }

    /// Insert a sprite with a particular index.
    #[inline]
    pub fn insert_at(&mut self, sprite_id: SpriteId, instance: Instance) -> Option<Instance> {
        self.dirty = true;
        self.sprites.insert_at(sprite_id.0, instance)
    }

    /// Insert a sprite at a given slot. Useful if the [`SpriteBatch`] is being used as a mostly
    /// dense but array of sprites.
    #[inline]
    pub fn insert_at_slot(
        &mut self,
        slot: u32,
        instance: Instance,
    ) -> (SpriteId, Option<Instance>) {
        self.dirty = true;
        let (index, old_instance) = self.sprites.insert_at_slot(slot, instance);
        (SpriteId(index), old_instance)
    }

    /// Borrow a sprite at a given slot, if present, ignoring its generation. If present, returns
    /// the corresponding sprite ID (with generational component.)
    #[inline]
    pub fn get_by_slot(&self, slot: u32) -> Option<(SpriteId, &Instance)> {
        self.sprites
            .get_by_slot(slot)
            .map(|(index, instance)| (SpriteId(index), instance))
    }

    /// Mutably borrow a sprite at a given slot, if present, ignoring its generation. If present,
    /// returns the corresponding sprite ID (with generational component.)
    #[inline]
    pub fn get_by_slot_mut(&mut self, slot: u32) -> Option<(SpriteId, &mut Instance)> {
        self.dirty = true;
        self.sprites
            .get_by_slot_mut(slot)
            .map(|(index, instance)| (SpriteId(index), instance))
    }

    /// Clear the spritebatch, removing all sprites in  it.
    #[inline]
    pub fn clear(&mut self) {
        self.dirty = true;
        self.sprites.clear();
    }

    /// Get a reference to the texture in this spritebatch.
    #[inline]
    pub fn texture(&self) -> &CachedTexture {
        &self.texture
    }

    /// Set the texture of this spritebatch directly. There should not often be a need for this.
    #[inline]
    pub fn set_texture(&mut self, texture: impl Into<CachedTexture>) {
        let mut new_texture = texture.into();

        if !CachedTexture::ptr_eq_cached(&mut self.texture, &mut new_texture) {
            self.dirty = true;
            self.texture = new_texture;
        }
    }

    /// Update the underlying GPU instance buffer with the current sprite data. This is called
    /// automatically by [`DrawableMut::draw_mut`], and is why [`SpriteBatch`] does not implement
    /// [`Drawable`].
    pub fn flush(&mut self, ctx: &mut Graphics) {
        let texture = self.texture.get_cached();

        if !self.dirty && texture.handle == self.bindings.images[0] {
            return;
        }

        self.instances.clear();
        self.instances.extend(self.sprites.iter().map(|(_, param)| {
            param
                .scale2(param.src.extents())
                .scale2(Vector2::new(
                    texture.width() as f32,
                    texture.height() as f32,
                ))
                .to_instance_properties()
        }));

        if self.instances.len() > self.capacity {
            let new_capacity = self.instances.len().checked_next_power_of_two().unwrap();
            let new_buffer = mq::Buffer::stream(
                &mut ctx.mq,
                mq::BufferType::VertexBuffer,
                new_capacity * mem::size_of::<InstanceProperties>(),
            );

            let old_buffer = mem::replace(&mut self.bindings.vertex_buffers[1], new_buffer);
            old_buffer.delete();

            self.capacity = new_capacity;
        }

        self.bindings.vertex_buffers[1].update(&mut ctx.mq, &self.instances);
        self.bindings.images[0] = texture.handle;

        self.dirty = false;
    }

    /// Get an iterator immutably borrowing the instances in this batch.
    pub fn iter(&self) -> SpriteBatchIter<'_> {
        SpriteBatchIter {
            iter: self.sprites.iter(),
        }
    }

    /// Get an iterator mutably borrowing the instances in this batch.
    pub fn iter_mut(&mut self) -> SpriteBatchIterMut<'_> {
        SpriteBatchIterMut {
            iter: self.sprites.iter_mut(),
        }
    }
}

/// TODO: FIXME(sleffy) maybe? This implementation ignores the color and src parameters
/// of the `InstanceParam`. Not sure there's much to be done about that, though, since
/// the spritebatch has its own instance parameters.
impl DrawableMut for SpriteBatch {
    fn draw_mut(&mut self, ctx: &mut Graphics, instance: Instance) {
        self.flush(ctx);

        ctx.modelview_mut().push(None);
        ctx.modelview_mut()
            .apply_transform(instance.tx.to_homogeneous());
        ctx.mq.apply_bindings(&self.bindings);
        ctx.apply_modelview();
        // 6 here because a quad is 6 vertices
        ctx.mq.draw(0, 6, self.instances.len() as i32);
        ctx.modelview_mut().pop();
        ctx.apply_modelview();
    }
}

impl LuaUserData for SpriteBatch {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        crate::lua::add_drawable_methods(methods);

        methods.add_method_mut("insert", |_, this, instance| Ok(this.insert(instance)));

        methods.add_method_mut("remove", |_, this, sprite_id| {
            this.remove(sprite_id);
            Ok(())
        });

        methods.add_method_mut("clear", |_, this, ()| {
            this.clear();
            Ok(())
        });

        methods.add_meta_method(LuaMetaMethod::Index, |_, this, sprite_id| {
            Ok(this[sprite_id])
        });

        methods.add_meta_method_mut(LuaMetaMethod::NewIndex, |_, this, (sprite_id, instance)| {
            this[sprite_id] = instance;
            Ok(())
        });

        methods.add_method_mut("set_texture", |_, this, texture: CachedTexture| {
            this.set_texture(texture);
            Ok(())
        });

        methods.add_method("texture", |_, this, ()| Ok(this.texture().clone()));
    }
}

#[derive(
    Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(transparent)]
pub struct TagId(pub u32);

impl<'lua> ToLua<'lua> for TagId {
    fn to_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        self.0.to_lua(lua)
    }
}

impl<'lua> FromLua<'lua> for TagId {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        u32::from_lua(lua_value, lua).map(TagId)
    }
}

#[derive(
    Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(transparent)]
pub struct FrameId(pub u32);

impl<'lua> ToLua<'lua> for FrameId {
    fn to_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        self.0.to_lua(lua)
    }
}

impl<'lua> FromLua<'lua> for FrameId {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        u32::from_lua(lua_value, lua).map(FrameId)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Direction {
    Forward,
    Reverse,
    Pingpong,
}

impl From<aseprite::Direction> for Direction {
    fn from(ad: aseprite::Direction) -> Self {
        match ad {
            aseprite::Direction::Forward => Self::Forward,
            aseprite::Direction::Reverse => Self::Reverse,
            aseprite::Direction::Pingpong => Self::Pingpong,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: Option<String>,
    pub from: FrameId,
    pub to: FrameId,
    pub direction: Direction,
}

#[derive(Debug, Clone, Copy)]
pub enum NextFrame {
    /// Returned if this is just the next frame ID.
    Stepped(FrameId),
    Wrapped(FrameId),
}

impl Tag {
    pub fn first_frame(&self) -> FrameId {
        match self.direction {
            Direction::Forward | Direction::Pingpong => self.from,
            Direction::Reverse => self.to,
        }
    }

    pub fn last_frame(&self) -> FrameId {
        match self.direction {
            Direction::Forward | Direction::Pingpong => self.to,
            Direction::Reverse => self.from,
        }
    }

    /// Returns `Err` if this next frame would loop the animation, `Ok` otherwise.
    pub fn next_frame(&self, current: FrameId, is_ponged: bool) -> Result<FrameId, FrameId> {
        match self.direction {
            Direction::Forward if current == self.to => Err(self.from),
            Direction::Reverse if current == self.from => Err(self.to),
            Direction::Pingpong if current == self.to => {
                Err(FrameId(na::max(self.to.0 - 1, self.from.0)))
            }
            Direction::Pingpong if current == self.from => {
                Err(FrameId(na::min(self.from.0 + 1, self.to.0)))
            }
            Direction::Forward => Ok(FrameId(current.0 + 1)),
            Direction::Reverse => Ok(FrameId(current.0 - 1)),
            Direction::Pingpong => match is_ponged {
                false => Ok(FrameId(current.0 + 1)),
                true => Ok(FrameId(current.0 - 1)),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FrameSource {
    pub frame: Box2<u32>,
    pub frame_source: Box2<u32>,
    pub source_size: Vector2<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Frame {
    pub source: Option<FrameSource>,
    pub offset: Vector2<f32>,
    pub uvs: Box2<f32>,
    pub duration: u32,
}

impl Frame {
    pub fn to_instance(&self) -> Instance {
        Instance::new()
            .src(self.uvs)
            .translate2(self.offset)
            .scale2(self.uvs.extents())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteSheetSource {
    pub image: Option<String>,
    pub size: Vector2<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteSheet {
    pub source: Option<SpriteSheetSource>,
    pub tag_ids: HashMap<String, TagId>,
    pub tags: Vec<Tag>,
    pub frames: Vec<Frame>,
}

impl ops::Index<TagId> for SpriteSheet {
    type Output = Tag;

    fn index(&self, TagId(id): TagId) -> &Self::Output {
        &self.tags[id as usize]
    }
}

impl ops::Index<SpriteTag> for SpriteSheet {
    type Output = Tag;

    fn index(&self, sprite_tag: SpriteTag) -> &Self::Output {
        &self[sprite_tag.tag_id]
    }
}

impl ops::Index<FrameId> for SpriteSheet {
    type Output = Frame;

    fn index(&self, FrameId(id): FrameId) -> &Self::Output {
        &self.frames[id as usize]
    }
}

impl ops::Index<SpriteFrame> for SpriteSheet {
    type Output = Frame;

    fn index(&self, SpriteFrame(FrameId(id)): SpriteFrame) -> &Self::Output {
        &self.frames[id as usize]
    }
}

impl Default for SpriteSheet {
    fn default() -> Self {
        Self::new()
    }
}

impl SpriteSheet {
    /// Create a new, empty spritesheet.
    pub fn new() -> Self {
        Self {
            source: None,
            tag_ids: HashMap::new(),
            tags: vec![Tag {
                name: None,
                from: FrameId(0),
                to: FrameId(0),
                direction: Direction::Forward,
            }],
            frames: vec![Frame {
                source: None,
                uvs: Box2::new(0., 0., 1., 1.),
                offset: Vector2::zeros(),
                duration: 1,
            }],
        }
    }

    /// Get the ID which will be returned by the next call to `insert_frame`. Useful for
    /// constructing new tags programmatically; call this to get the index of the first frame of the
    /// tag before you insert a bunch of frames in sequence, and then call [`last_frame_id`] to get
    /// the last ID of the tag, giving you the `from` and `to` fields needed to construct a [`Tag`].
    pub fn next_frame_id(&self) -> FrameId {
        FrameId(self.frames.len() as u32)
    }

    /// Get the ID of the very last frame currently in the spritesheet. Will panic if the
    /// spritesheet has no frames in it.
    pub fn last_frame_id(&self) -> FrameId {
        assert!(!self.frames.is_empty());
        FrameId((self.frames.len() - 1) as u32)
    }

    /// Insert a new frame and get back its "frame ID". Frame IDs are created sequentially; the
    /// `u32` inside will always be the next `u32` after the previously returned `FrameId`. This is
    /// very important because tags deal with *ranges* of `FrameId`s.
    pub fn insert_frame(&mut self, frame: Frame) -> FrameId {
        let id = self.frames.len();
        self.frames.push(frame);
        FrameId(id as u32)
    }

    /// Insert a new tag and get back its "tag ID". Like frame IDs, tag IDs are also created
    /// sequentially, but we care less because we don't deal with ranges of tags.
    pub fn insert_tag(&mut self, tag: Tag) -> TagId {
        let tag_id = TagId(self.tags.len() as u32);
        if let Some(name) = tag.name.clone() {
            self.tag_ids.insert(name, tag_id);
        }
        self.tags.push(tag);
        tag_id
    }

    pub fn from_reader<R: Read>(reader: &mut R) -> Result<Self> {
        let mut buf = String::new();
        reader.read_to_string(&mut buf)?;
        Self::from_json(&buf)
    }

    pub fn from_json(s: &str) -> Result<Self> {
        let spritesheet_data = serde_json::from_str::<SpritesheetData>(s)?;
        let dims = spritesheet_data.meta.size;
        let size = Vector2::new(dims.w, dims.h);

        let mut frames = Vec::new();
        for ase_frame in spritesheet_data.frames.into_iter() {
            let fr = ase_frame.frame;
            let sb = ase_frame.sprite_source_size;
            let ss = ase_frame.source_size;

            let duration = ase_frame.duration;
            let frame = Box2::new(fr.x, fr.y, fr.w, fr.h);
            let frame_source = Box2::new(sb.x, sb.y, sb.w, sb.h);
            let source_size = Vector2::new(ss.w, ss.h);
            // `bw` is border width. only nonzero if there's padding added to the spritesheet.
            let bw = Vector2::new(fr.w - sb.w, fr.h - sb.h).cast::<i32>() / 2;
            let offset = Vector2::new(sb.x as i32 - bw.x, sb.y as i32 - bw.y).cast::<f32>()
                - Vector2::new(ss.w, ss.h).cast::<f32>() / 2.;
            let uvs = Box2::new(
                fr.x as f32 / size.x as f32,
                fr.y as f32 / size.y as f32,
                fr.w as f32 / size.x as f32,
                fr.h as f32 / size.y as f32,
            );

            frames.push(Frame {
                source: Some(FrameSource {
                    frame,
                    frame_source,
                    source_size,
                }),
                offset,
                uvs,
                duration,
            });
        }

        let mut tags = vec![Tag {
            name: None,
            from: FrameId(0),
            to: FrameId(frames.len() as u32 - 1),
            direction: Direction::Forward,
        }];

        for frame_tag in spritesheet_data.meta.frame_tags.into_iter().flatten() {
            tags.push(Tag {
                name: Some(frame_tag.name),
                from: FrameId(frame_tag.from),
                to: FrameId(frame_tag.to),
                direction: Direction::from(frame_tag.direction),
            });
        }

        let tag_ids = tags
            .iter()
            .enumerate()
            .filter_map(|(i, tag)| tag.name.clone().map(|n| (n, TagId(i as u32))))
            .collect::<HashMap<_, _>>();

        Ok(Self {
            source: Some(SpriteSheetSource {
                image: spritesheet_data.meta.image,
                size,
            }),
            tag_ids,
            tags,
            frames,
        })
    }

    pub fn update_animation(&self, dt: f32, tag: &mut SpriteTag, frame: &mut SpriteFrame) {
        if let Some((new_tag, maybe_new_frame)) = self.update_animation_inner(dt, tag, frame) {
            *tag = new_tag;

            if let Some(new_frame) = maybe_new_frame {
                *frame = new_frame;
            }
        }
    }

    fn update_animation_inner(
        &self,
        dt: f32,
        tag_state: &SpriteTag,
        SpriteFrame(frame): &SpriteFrame,
    ) -> Option<(SpriteTag, Option<SpriteFrame>)> {
        if !tag_state.is_paused {
            let mut new_tag_state = SpriteTag {
                remaining: tag_state.remaining - dt * 1_000.,
                ..*tag_state
            };

            if new_tag_state.remaining < 0. {
                let tag = &self[new_tag_state.tag_id];
                match tag.next_frame(*frame, new_tag_state.is_ponged) {
                    Err(_) if !tag_state.should_loop => Some((
                        SpriteTag {
                            is_paused: true,
                            ..new_tag_state
                        },
                        Some(SpriteFrame(self[new_tag_state.tag_id].last_frame())),
                    )),
                    result @ (Ok(new_frame) | Err(new_frame)) => {
                        if matches!(tag.direction, Direction::Pingpong) && result.is_err() {
                            // If we wrapped and this tag is set to ping-pong, then we need to flip
                            // the direction.
                            new_tag_state.is_ponged = !new_tag_state.is_ponged;
                        }

                        new_tag_state.remaining += self[new_frame].duration as f32;
                        Some((new_tag_state, Some(SpriteFrame(new_frame))))
                    }
                }
            } else {
                Some((new_tag_state, None))
            }
        } else {
            None
        }
    }

    pub fn get_tag<K: AsRef<str>>(&self, s: K) -> Option<TagId> {
        self.tag_ids.get(s.as_ref()).copied()
    }

    pub fn at_tag(&self, tag_id: TagId, should_loop: bool) -> (SpriteFrame, SpriteTag) {
        let tag = &self[tag_id];
        let ff = tag.first_frame();
        (
            SpriteFrame(ff),
            SpriteTag {
                tag_id,
                remaining: self[ff].duration as f32,
                is_paused: false,
                should_loop,
                is_ponged: false,
            },
        )
    }
}

#[derive(Debug, Clone)]
pub struct CachedSpriteSheet {
    handle: CacheRef<SpriteSheet>,
}

impl CachedSpriteSheet {
    pub fn new_uncached(sprite_sheet: SpriteSheet) -> Self {
        Self {
            handle: CacheRef::new_uncached(sprite_sheet),
        }
    }

    pub fn get(&self) -> Guard<SpriteSheet> {
        self.handle.get()
    }

    pub fn get_cached(&mut self) -> &SpriteSheet {
        self.handle.get_cached()
    }
}

impl LuaUserData for CachedSpriteSheet {}

/// Component holding the string name of a spritesheet.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SpriteName(pub String);

/// Component holding the current frame ID of a sprite.
#[derive(Debug, Copy, Clone, Default)]
pub struct SpriteFrame(pub FrameId);

/// Component holding the state of a running animation at a given tag.
#[derive(Debug, Copy, Clone)]
pub struct SpriteTag {
    /// The index of the currently running animation/tag.
    pub tag_id: TagId,
    /// Remaining time for this frame, in milliseconds.
    pub remaining: f32,
    /// Whether this animation is running or paused.
    pub is_paused: bool,
    /// Whether this animation should loop, or pause on the last frame.
    pub should_loop: bool,
    /// Whether this animation is going forward or backward; only used for `PingPong` direction, to
    /// store the state of which direction we're currently going.
    pub is_ponged: bool,
}

impl Default for SpriteTag {
    fn default() -> Self {
        Self {
            tag_id: TagId::default(),
            remaining: 0.,
            is_paused: false,
            should_loop: true,
            is_ponged: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpriteAnimationState {
    pub sheet: CachedSpriteSheet,
    pub frame: SpriteFrame,
    pub tag: SpriteTag,
}

impl LuaUserData for SpriteAnimationState {}

pub struct FilesystemSpriteSheetLoader {
    engine: EngineRef,
}

impl FilesystemSpriteSheetLoader {
    pub fn new(engine: &Engine) -> Self {
        Self {
            engine: engine.downgrade(),
        }
    }
}

impl<P: AsRef<Path>> Loader<P, SpriteSheet> for FilesystemSpriteSheetLoader {
    fn load(&mut self, key: &P) -> Result<Handle<SpriteSheet>> {
        let engine = self.engine.upgrade();
        let mut file = engine.fs().open(key)?;
        let sprite_sheet = SpriteSheet::from_reader(&mut file)?;
        Ok(Handle::new(sprite_sheet))
    }
}

pub struct SpriteSheetCache {
    inner: SwappableCache<String, SpriteSheet, FilesystemSpriteSheetLoader>,
}

impl LuaUserData for SpriteSheetCache {}

impl LuaResource for SpriteSheetCache {
    const REGISTRY_KEY: &'static str = "HV_FRIENDS_SPRITE_SHEET_CACHE";
}

impl SpriteSheetCache {
    pub fn new(engine: &Engine) -> Self {
        Self {
            inner: SwappableCache::new(FilesystemSpriteSheetLoader::new(engine)),
        }
    }

    pub fn get_or_load(&mut self, path: impl Into<String>) -> Result<CachedSpriteSheet> {
        Ok(CachedSpriteSheet {
            handle: self.inner.get_or_load(path.into())?.into_cached(),
        })
    }

    pub fn reload_all(&mut self) -> Result<()> {
        self.inner.reload_all()
    }
}
