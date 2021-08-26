use egui::paint::Vertex;
use hv_core::{engine::Engine, mq};

pub struct Painter {
    pipeline: mq::Pipeline,
    bindings: mq::Bindings,
    egui_texture_version: u64,
    egui_texture: mq::Texture,
}

impl Painter {
    pub fn new(engine: &Engine) -> Painter {
        let ctx = &mut engine.mq();
        let shader = mq::Shader::new(ctx, shader::VERTEX, shader::FRAGMENT, shader::meta());

        let pipeline = mq::Pipeline::with_params(
            ctx,
            &[mq::BufferLayout::default()],
            &[
                mq::VertexAttribute::new("a_pos", mq::VertexFormat::Float2),
                mq::VertexAttribute::new("a_tc", mq::VertexFormat::Float2),
                mq::VertexAttribute::new("a_srgba", mq::VertexFormat::Byte4),
            ],
            shader.expect("couldn't make shader"),
            mq::PipelineParams {
                color_blend: Some(mq::BlendState::new(
                    mq::Equation::Add,
                    mq::BlendFactor::One,
                    mq::BlendFactor::OneMinusValue(mq::BlendValue::SourceAlpha),
                )),
                ..Default::default()
            },
        );

        let vertex_buffer = mq::Buffer::stream(
            ctx,
            mq::BufferType::VertexBuffer,
            32 * 1024 * std::mem::size_of::<Vertex>(),
        );
        let index_buffer = mq::Buffer::stream(
            ctx,
            mq::BufferType::IndexBuffer,
            32 * 1024 * std::mem::size_of::<u16>(),
        );

        let bindings = mq::Bindings {
            vertex_buffers: vec![vertex_buffer],
            index_buffer,
            images: vec![mq::Texture::empty()],
        };

        Painter {
            pipeline,
            bindings,
            egui_texture_version: 0,
            egui_texture: mq::Texture::empty(),
        }
    }

    fn rebuild_egui_texture(&mut self, ctx: &mut mq::Context, texture: &egui::Texture) {
        self.egui_texture.delete();

        let mut texture_data = Vec::new();
        for pixel in texture.srgba_pixels(1.0) {
            texture_data.push(pixel.r());
            texture_data.push(pixel.g());
            texture_data.push(pixel.b());
            texture_data.push(pixel.a());
        }
        assert_eq!(texture_data.len(), texture.width * texture.height * 4);
        self.egui_texture = mq::Texture::from_data_and_format(
            ctx,
            &texture_data,
            mq::TextureParams {
                format: mq::TextureFormat::RGBA8,
                wrap: mq::TextureWrap::Clamp,
                filter: mq::FilterMode::Linear,
                width: texture.width as _,
                height: texture.height as _,
            },
        );
    }

    pub fn paint(
        &mut self,
        engine: &Engine,
        meshes: Vec<egui::ClippedMesh>,
        texture: &egui::Texture,
    ) {
        let mq = &mut engine.mq();

        if texture.version != self.egui_texture_version {
            self.rebuild_egui_texture(mq, texture);
            self.egui_texture_version = texture.version;
        }

        mq.begin_default_pass(mq::PassAction::Nothing);
        mq.apply_pipeline(&self.pipeline);

        let screen_size_in_pixels = mq.screen_size();
        let screen_size_in_points = (
            screen_size_in_pixels.0 / mq.dpi_scale(),
            screen_size_in_pixels.1 / mq.dpi_scale(),
        );
        mq.apply_uniforms(&shader::Uniforms {
            u_screen_size: screen_size_in_points,
        });

        for egui::ClippedMesh(clip_rect, mesh) in meshes {
            self.paint_job(mq, clip_rect, mesh);
        }

        mq.end_render_pass();
    }

    fn paint_job(&mut self, mq: &mut mq::Context, clip_rect: egui::Rect, mesh: egui::paint::Mesh) {
        let screen_size_in_pixels = mq.screen_size();
        let pixels_per_point = mq.dpi_scale();

        // TODO: support u32 indices in mq and just use "mesh.indices" without a need for `split_to_u16`
        let meshes = mesh.split_to_u16();
        for mesh in meshes {
            assert!(mesh.is_valid());
            let vertices_size_bytes = mesh.vertices.len() * std::mem::size_of::<Vertex>();
            if self.bindings.vertex_buffers[0].size() < vertices_size_bytes {
                self.bindings.vertex_buffers[0].delete();
                self.bindings.vertex_buffers[0] =
                    mq::Buffer::stream(mq, mq::BufferType::VertexBuffer, vertices_size_bytes);
            }
            self.bindings.vertex_buffers[0].update(mq, &mesh.vertices);

            let indices_size_bytes = mesh.indices.len() * std::mem::size_of::<u16>();
            if self.bindings.index_buffer.size() < indices_size_bytes {
                self.bindings.index_buffer.delete();
                self.bindings.index_buffer =
                    mq::Buffer::stream(mq, mq::BufferType::IndexBuffer, indices_size_bytes);
            }
            self.bindings.index_buffer.update(mq, &mesh.indices);

            self.bindings.images[0] = match mesh.texture_id {
                egui::TextureId::Egui => self.egui_texture,
                egui::TextureId::User(id) => unsafe { mq::Texture::from_raw_id(id as u32) },
            };

            let (width_in_pixels, height_in_pixels) = screen_size_in_pixels;

            // From https://github.com/emilk/egui/blob/master/egui_glium/src/painter.rs#L233

            // Transform clip rect to physical pixels:
            let clip_min_x = pixels_per_point * clip_rect.min.x;
            let clip_min_y = pixels_per_point * clip_rect.min.y;
            let clip_max_x = pixels_per_point * clip_rect.max.x;
            let clip_max_y = pixels_per_point * clip_rect.max.y;

            // Make sure clip rect can fit withing an `u32`:
            let clip_min_x = clip_min_x.clamp(0.0, width_in_pixels as f32);
            let clip_min_y = clip_min_y.clamp(0.0, height_in_pixels as f32);
            let clip_max_x = clip_max_x.clamp(clip_min_x, width_in_pixels as f32);
            let clip_max_y = clip_max_y.clamp(clip_min_y, height_in_pixels as f32);

            let clip_min_x = clip_min_x.round() as u32;
            let clip_min_y = clip_min_y.round() as u32;
            let clip_max_x = clip_max_x.round() as u32;
            let clip_max_y = clip_max_y.round() as u32;

            mq.apply_scissor_rect(
                clip_min_x as i32,
                (height_in_pixels as u32 - clip_max_y) as i32,
                (clip_max_x - clip_min_x) as i32,
                (clip_max_y - clip_min_y) as i32,
            );
            mq.apply_bindings(&self.bindings);
            mq.draw(0, mesh.indices.len() as i32, 1);
        }
    }
}

mod shader {
    use hv_core::mq::{ShaderMeta, UniformBlockLayout, UniformDesc, UniformType};

    pub const VERTEX: &str = r#"
    #version 100
    uniform vec2 u_screen_size;
    attribute vec2 a_pos;
    attribute vec2 a_tc;
    attribute vec4 a_srgba;
    varying vec2 v_tc;
    varying vec4 v_rgba;
    // 0-1 linear  from  0-255 sRGB
    vec3 linear_from_srgb(vec3 srgb) {
        bvec3 cutoff = lessThan(srgb, vec3(10.31475));
        vec3 lower = srgb / vec3(3294.6);
        vec3 higher = pow((srgb + vec3(14.025)) / vec3(269.025), vec3(2.4));
        return mix(higher, lower, vec3(cutoff));
    }
    // 0-1 linear  from  0-255 sRGBA
    vec4 linear_from_srgba(vec4 srgba) {
        return vec4(linear_from_srgb(srgba.rgb), srgba.a / 255.0);
    }
    void main() {
        gl_Position = vec4(
            2.0 * a_pos.x / u_screen_size.x - 1.0,
            1.0 - 2.0 * a_pos.y / u_screen_size.y,
            0.0,
            1.0);
        v_tc = a_tc;
        v_rgba = linear_from_srgba(a_srgba);
    }
    "#;

    pub const FRAGMENT: &str = r#"
    #version 100
    uniform sampler2D u_sampler;
    precision highp float;
    varying vec2 v_tc;
    varying vec4 v_rgba;
    // 0-1 linear  from  0-255 sRGB
    vec3 linear_from_srgb(vec3 srgb) {
        bvec3 cutoff = lessThan(srgb, vec3(10.31475));
        vec3 lower = srgb / vec3(3294.6);
        vec3 higher = pow((srgb + vec3(14.025)) / vec3(269.025), vec3(2.4));
        return mix(higher, lower, vec3(cutoff));
    }
    // 0-1 linear  from  0-255 sRGBA
    vec4 linear_from_srgba(vec4 srgba) {
        return vec4(linear_from_srgb(srgba.rgb), srgba.a / 255.0);
    }
    // 0-255 sRGB  from  0-1 linear
    vec3 srgb_from_linear(vec3 rgb) {
        bvec3 cutoff = lessThan(rgb, vec3(0.0031308));
        vec3 lower = rgb * vec3(3294.6);
        vec3 higher = vec3(269.025) * pow(rgb, vec3(1.0 / 2.4)) - vec3(14.025);
        return mix(higher, lower, vec3(cutoff));
    }
    // 0-255 sRGBA  from  0-1 linear
    vec4 srgba_from_linear(vec4 rgba) {
        return vec4(srgb_from_linear(rgba.rgb), 255.0 * rgba.a);
    }
    void main() {
        vec4 texture_srgba = texture2D(u_sampler, v_tc);
        vec4 texture_rgba = linear_from_srgba(texture2D(u_sampler, v_tc) * 255.0); // TODO: sRGBA aware sampeler, see linear_from_srgb;
        gl_FragColor = v_rgba * texture_rgba;
        // mq doesn't support linear blending in the framebuffer.
        // so we need to convert linear to sRGBA:
        gl_FragColor = srgba_from_linear(gl_FragColor) / 255.0; // TODO: sRGBA aware framebuffer
        // We also apply this hack to at least get a bit closer to the desired blending:
        gl_FragColor.a = pow(gl_FragColor.a, 1.6); // Empiric nonsense
    }
    "#;

    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec!["u_sampler".to_string()],
            uniforms: UniformBlockLayout {
                uniforms: vec![UniformDesc::new("u_screen_size", UniformType::Float2)],
            },
        }
    }

    #[repr(C)]
    #[derive(Debug)]
    pub struct Uniforms {
        pub u_screen_size: (f32, f32),
    }
}
