local std_space = require("std.space")
local binser = require("std.binser")
local Position = hf.components.Position

local objects = {}

-- Load map?

local LevelObject = std_space.Object:extend("smb1_1.LevelObject")
    :with(Position)
do
    objects.LevelObject = LevelObject
    binser.registerClass(LevelObject)
    
    function LevelObject:init(space, x, y, ...)
        LevelObject.super.init(self, space, Position(x, y), ...)
    end
end

local Goomba = LevelObject:extend("smb1_1.Goomba")
do
    objects.Goomba = Goomba
    binser.registerClass(Goomba)

    function Goomba:spawn(space)
        error("TODO!")
    end
end

local Koopa = LevelObject:extend("smb1_1.Koopa")
do
    objects.Koopa = Koopa
    binser.registerClass(Koopa)

    function Koopa:spawn(space)
        error("TODO!")
    end
end

return objects