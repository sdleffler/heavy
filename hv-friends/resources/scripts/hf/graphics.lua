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

return {
    load_texture_from_filesystem = hf_graphics.load_texture_from_filesystem,
    load_sprite_sheet_from_filesystem = hf_graphics.load_sprite_sheet_from_filesystem,

    reload_textures_and_sprite_sheets = function() reload_textures(); reload_sprite_sheets() end,

    Drawable = Drawable,
    Instance = hf_graphics.create_instance_object,
    SpriteBatch = hf_graphics.create_sprite_batch_object,

    nil
}