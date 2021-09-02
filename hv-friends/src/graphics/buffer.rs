use hv_core::{
    engine::{LuaExt, WeakResourceCache},
    mq,
    prelude::*,
};
use std::{io::Write, ops::Deref, sync::Arc};

use crate::graphics::{Graphics, GraphicsLock, GraphicsLockExt};

#[derive(Clone, Copy, Debug)]
pub enum BufferElement {
    Float1,
    Float2,
    Float3,
    Float4,
    Byte1,
    Byte2,
    Byte3,
    Byte4,
    Short1,
    Short2,
    Short3,
    Short4,
    Int1,
    Int2,
    Int3,
    Int4,
    Mat4,
}

impl BufferElement {
    pub fn datum_count(&self) -> usize {
        match self {
            BufferElement::Float1 => 1,
            BufferElement::Float2 => 2,
            BufferElement::Float3 => 3,
            BufferElement::Float4 => 4,
            BufferElement::Byte1 => 1,
            BufferElement::Byte2 => 2,
            BufferElement::Byte3 => 3,
            BufferElement::Byte4 => 4,
            BufferElement::Short1 => 1,
            BufferElement::Short2 => 2,
            BufferElement::Short3 => 3,
            BufferElement::Short4 => 4,
            BufferElement::Int1 => 1,
            BufferElement::Int2 => 2,
            BufferElement::Int3 => 3,
            BufferElement::Int4 => 4,
            BufferElement::Mat4 => 16,
        }
    }
}

impl LuaUserData for BufferElement {}

#[derive(Debug, Clone)]
pub struct BufferFormat {
    pub types: Arc<[BufferElement]>,
}

impl BufferFormat {
    pub fn datum_count(&self) -> usize {
        self.types.iter().map(BufferElement::datum_count).sum()
    }

    pub fn byte_size_with_padding(&self) -> usize {
        use crevice::std430::*;

        let mut sizer = Sizer::new();

        // We loop around once so that the total size includes the padding.
        let mut last_size = 0;
        for ty in self.types.iter().chain(self.types.first()) {
            last_size = match ty {
                BufferElement::Float1 => sizer.add::<f32>(),
                BufferElement::Float2 => sizer.add::<Vec2>(),
                BufferElement::Float3 => sizer.add::<Vec3>(),
                BufferElement::Float4 => sizer.add::<Vec4>(),
                BufferElement::Mat4 => sizer.add::<Mat4>(),
                _ => unimplemented!(),
            };
        }

        last_size
    }

    pub fn write_lua_table_to_bytes_with_format<W: Write>(
        &self,
        lua: &Lua,
        data: LuaTable,
        write: W,
    ) -> Result<()> {
        use crevice::std430::*;

        let datum_count = self.datum_count();
        let mut writer = Writer::new(write);

        for v in data.sequence_values() {
            let vertex_table: LuaTable = v?;
            assert_eq!(vertex_table.len()? as usize, datum_count);

            let mut seq = vertex_table.sequence_values::<LuaValue>();
            for ty in self.types.iter() {
                match ty {
                    BufferElement::Float1 => {
                        writer.write_std430::<f32>(&lua.from_value(seq.next().unwrap()?)?)?;
                    }
                    BufferElement::Float2 => {
                        let mut extract_float =
                            || LuaResult::Ok(lua.from_value::<f32>(seq.next().unwrap()?)?);
                        writer.write_std430(&Vec2 {
                            x: extract_float()?,
                            y: extract_float()?,
                        })?;
                    }
                    BufferElement::Float3 => {
                        let mut extract_float =
                            || LuaResult::Ok(lua.from_value::<f32>(seq.next().unwrap()?)?);
                        writer.write_std430(&Vec3 {
                            x: extract_float()?,
                            y: extract_float()?,
                            z: extract_float()?,
                        })?;
                    }
                    BufferElement::Float4 => {
                        let mut extract_float =
                            || LuaResult::Ok(lua.from_value::<f32>(seq.next().unwrap()?)?);
                        writer.write_std430(&Vec4 {
                            x: extract_float()?,
                            y: extract_float()?,
                            z: extract_float()?,
                            w: extract_float()?,
                        })?;
                    }
                    BufferElement::Mat4 => {
                        let mut extract_float =
                            || LuaResult::Ok(lua.from_value::<f32>(seq.next().unwrap()?)?);

                        let mut extract_vec4 = || {
                            LuaResult::Ok(Vec4 {
                                x: extract_float()?,
                                y: extract_float()?,
                                z: extract_float()?,
                                w: extract_float()?,
                            })
                        };

                        let mat4 = Mat4 {
                            x: extract_vec4()?,
                            y: extract_vec4()?,
                            z: extract_vec4()?,
                            w: extract_vec4()?,
                        };

                        writer.write_std430(&mat4)?;
                    }
                    _ => unimplemented!(),
                }
            }
        }

        Ok(())
    }
}

impl LuaUserData for BufferFormat {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BufferType {
    VertexBuffer,
    IndexBuffer,
}

impl From<BufferType> for mq::BufferType {
    fn from(buffer_ty: BufferType) -> Self {
        match buffer_ty {
            BufferType::VertexBuffer => mq::BufferType::VertexBuffer,
            BufferType::IndexBuffer => mq::BufferType::IndexBuffer,
        }
    }
}

impl LuaUserData for BufferType {}

#[derive(Debug)]
pub struct OwnedBuffer {
    pub handle: mq::Buffer,
    pub mutable: bool,
    pub buffer_type: BufferType,
    pub format: Option<BufferFormat>,
}

impl OwnedBuffer {
    pub fn immutable<T>(gfx: &mut Graphics, buffer_type: BufferType, data: &[T]) -> Self {
        let handle = mq::Buffer::immutable(&mut gfx.mq, buffer_type.into(), data);
        Self {
            handle,
            mutable: false,
            buffer_type,
            format: None,
        }
    }

    pub fn immutable_with_format<T: AsRef<[u8]>>(
        gfx: &mut Graphics,
        buffer_type: BufferType,
        format: BufferFormat,
        data: &T,
    ) -> Self {
        let bytes = data.as_ref();
        assert_eq!(bytes.len() % format.byte_size_with_padding(), 0);

        let buffer = match buffer_type {
            BufferType::IndexBuffer => {
                // Cut these bytes short, but actually don't; this is an alignment check. It will
                // never fail unless somehow a single-byte-aligned slice gets in here trying to look
                // like u16s.
                let jorts = {
                    let (head, body, tail) = unsafe { bytes.align_to::<u16>() };
                    assert!(head.is_empty() && tail.is_empty());
                    body
                };

                mq::Buffer::immutable(&mut gfx.mq, mq::BufferType::IndexBuffer, jorts)
            }
            BufferType::VertexBuffer => {
                mq::Buffer::immutable(&mut gfx.mq, mq::BufferType::VertexBuffer, bytes)
            }
        };

        OwnedBuffer::from_inner(buffer, false, buffer_type, Some(format))
    }

    pub fn streaming(gfx: &mut Graphics, buffer_type: BufferType, size: usize) -> OwnedBuffer {
        let handle = mq::Buffer::stream(&mut gfx.mq, buffer_type.into(), size);
        Self {
            handle,
            mutable: true,
            buffer_type,
            format: None,
        }
    }

    pub fn streaming_with_format(
        gfx: &mut Graphics,
        buffer_type: BufferType,
        format: BufferFormat,
        len: usize,
    ) -> Self {
        let buffer = mq::Buffer::stream(
            &mut gfx.mq,
            buffer_type.into(),
            format.byte_size_with_padding() * len,
        );
        OwnedBuffer::from_inner(buffer, true, buffer_type, Some(format))
    }

    pub fn update<T>(&self, gfx: &mut Graphics, data: &[T]) {
        assert!(self.mutable, "attempted to update immutable buffer");
        self.handle.update(&mut gfx.mq, data);
    }

    pub fn update_formatted<T: AsRef<[u8]>>(&self, gfx: &mut Graphics, data: &T) {
        assert!(self.mutable, "attempted to update immutable buffer");
        let bytes = data.as_ref();
        let format = self
            .format
            .as_ref()
            .expect("tried to do formatted update of unformatted buffer");
        assert_eq!(bytes.len() % format.byte_size_with_padding(), 0);

        match self.buffer_type {
            BufferType::IndexBuffer => {
                // Cut these bytes short, but actually don't; this is an alignment check. It will
                // never fail unless somehow a single-byte-aligned slice gets in here trying to look
                // like u16s.
                let jorts = {
                    let (head, body, tail) = unsafe { bytes.align_to::<u16>() };
                    assert!(head.is_empty() && tail.is_empty());
                    body
                };

                self.update(gfx, jorts)
            }
            BufferType::VertexBuffer => self.update(gfx, bytes),
        };
    }

    pub fn format(&self) -> Option<&BufferFormat> {
        self.format.as_ref()
    }

    pub fn from_inner(
        mq_buf: mq::Buffer,
        mutable: bool,
        buffer_type: BufferType,
        format: Option<BufferFormat>,
    ) -> Self {
        Self {
            handle: mq_buf,
            mutable,
            buffer_type,
            format,
        }
    }
}

impl Drop for OwnedBuffer {
    fn drop(&mut self) {
        self.handle.delete();
    }
}

#[derive(Debug, Clone)]
pub struct Buffer {
    pub shared: Arc<OwnedBuffer>,
}

impl Deref for Buffer {
    type Target = OwnedBuffer;

    fn deref(&self) -> &Self::Target {
        &self.shared
    }
}

impl From<OwnedBuffer> for Buffer {
    fn from(owned: OwnedBuffer) -> Self {
        Self {
            shared: Arc::new(owned),
        }
    }
}

impl LuaUserData for Buffer {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        let mut weak_gfx_cache = WeakResourceCache::<GraphicsLock>::new();
        methods.add_method_mut("update", move |lua, this, data: LuaTable| {
            let gfx_lock = weak_gfx_cache.get(|| lua.get_resource())?;
            let mut bytes = Vec::new();
            this.format
                .as_ref()
                .expect("not a formatted buffer")
                .write_lua_table_to_bytes_with_format(lua, data, &mut bytes)
                .to_lua_err()?;
            this.update_formatted(&mut gfx_lock.lock(), &bytes);

            Ok(())
        });
    }
}

pub(super) fn open<'lua>(
    lua: &'lua Lua,
    gfx_lock: &Shared<GraphicsLock>,
) -> Result<LuaTable<'lua>, Error> {
    let buffer = lua.create_table()?;

    buffer.set(
        "BufferElement",
        lua.create_table_from(vec![
            ("Float1", BufferElement::Float1),
            ("Float2", BufferElement::Float2),
            ("Float3", BufferElement::Float3),
            ("Float4", BufferElement::Float4),
            ("Byte1", BufferElement::Byte1),
            ("Byte2", BufferElement::Byte2),
            ("Byte3", BufferElement::Byte3),
            ("Byte4", BufferElement::Byte4),
            ("Short1", BufferElement::Short1),
            ("Short2", BufferElement::Short2),
            ("Short3", BufferElement::Short3),
            ("Short4", BufferElement::Short4),
            ("Int1", BufferElement::Int1),
            ("Int2", BufferElement::Int2),
            ("Int3", BufferElement::Int3),
            ("Int4", BufferElement::Int4),
            ("Mat4", BufferElement::Mat4),
        ])?,
    )?;

    buffer.set(
        "BufferType",
        lua.create_table_from(vec![
            ("VertexBuffer", BufferType::VertexBuffer),
            ("IndexBuffer", BufferType::IndexBuffer),
        ])?,
    )?;

    let gfx = gfx_lock.clone();
    let mut scratch: Vec<u8> = Vec::new();
    buffer.set(
        "create_immutable_buffer_object",
        lua.create_function_mut(
            move |lua, (buffer_type, format, data): (BufferType, BufferFormat, LuaTable)| {
                scratch.clear();
                format
                    .write_lua_table_to_bytes_with_format(lua, data, &mut scratch)
                    .to_lua_err()?;
                Ok(Buffer::from(OwnedBuffer::immutable_with_format(
                    &mut gfx.lock(),
                    buffer_type,
                    format,
                    &scratch,
                )))
            },
        )?,
    )?;

    let gfx = gfx_lock.clone();
    buffer.set(
        "create_streaming_buffer_object",
        lua.create_function(
            move |_lua, (buffer_type, format, len): (BufferType, BufferFormat, usize)| {
                Ok(Buffer::from(OwnedBuffer::streaming_with_format(
                    &mut gfx.lock(),
                    buffer_type,
                    format,
                    len,
                )))
            },
        )?,
    )?;

    buffer.set(
        "create_buffer_format_object",
        lua.create_function(|_lua, elements: Vec<BufferElement>| {
            Ok(BufferFormat {
                types: Arc::from(elements),
            })
        })?,
    )?;

    Ok(buffer)
}
