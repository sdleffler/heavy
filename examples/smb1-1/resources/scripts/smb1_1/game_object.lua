local binser = require("std.binser")
local class = require("std.class")
local ObjectTable = hv.components.ObjectTable
local Position = hf.components.Position

local GameObject = class("smb1_1.GameObject"):with(Position)
do
    binser.registerClass(GameObject)

    function GameObject:init(space, x, y, ...)
        space:spawn(ObjectTable(self), Position(x, y), ...)
    end

    function GameObject:on_collide_with_object(other) end
end

return { GameObject = GameObject }
