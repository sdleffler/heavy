local hf_graphics = hv.plugins.friends.graphics
local class = require("std.class")

local Drawable
do
    Drawable = class("Drawable")

    function Drawable:draw(params) end
end

local Texture
do
    Texture = Drawable:extend("Texture")

    local load_texture_from_filesystem = hf_graphics.load_texture_from_filesystem
    function Texture:init(path)
        self._texture = load_texture_from_filesystem(path)
    end

    function Texture:draw(instance)
        self._texture:draw(instance)
    end
end

local reload_textures = hf_graphics.reload_textures
local reload_sprite_sheets = hf_graphics.reload_sprite_sheets

local SpriteAnimation = {}
do
    local hf_sprite = hf_graphics.sprite

    SpriteAnimation.new = assert(hf_sprite.create_sprite_animation)
end

return {
    load_texture_from_filesystem = hf_graphics.load_texture_from_filesystem,
    load_sprite_sheet_from_filesystem = hf_graphics.load_sprite_sheet_from_filesystem,

    reload_textures_and_sprite_sheets = function() reload_textures(); reload_sprite_sheets() end,
    
    SpriteAnimation = SpriteAnimation,

    Drawable = Drawable,
    Instance = hf_graphics.create_instance_object,
    SpriteBatch = hf_graphics.create_sprite_batch_object,

    circle = hf_graphics.circle,
    line = hf_graphics.line,
    points = hf_graphics.points,
    polygon = hf_graphics.polygon,
    print = hf_graphics.print,
    
    clear = hf_graphics.clear,
    present = hf_graphics.present,

    set_color = hf_graphics.set_color,

    apply_transform = hf_graphics.apply_transform,
    inverse_transform_point = hf_graphics.inverse_transform_point,
    origin = hf_graphics.origin,
    pop = hf_graphics.pop,
    push = hf_graphics.push,
    replace_transform = hf_graphics.replace_transform,
    rotate = hf_graphics.rotate,
    scale = hf_graphics.scale,
    shear = hf_graphics.shear,
    transform_point = hf_graphics.transform_point,
    translate = hf_graphics.translate,

    DrawMode = hf_graphics.DrawMode,
}