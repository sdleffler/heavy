local binser = require("std.binser")
local class = require("std.class")
local game_objects = {
    Goomba = require("smb1_1.goomba").Goomba,
    Koopa = require("smb1_1.koopa").Koopa,
    Player = require("smb1_1.player").Player,
}
local ObjectTable = hv.components.ObjectTable
local Position = hf.components.Position

local LevelObject = class("smb1_1.level_encoding.LevelObject")
    :with(Position)
do
    binser.registerClass(LevelObject)
    
    function LevelObject:init(space, x, y, ...)
        space:spawn(
            ObjectTable(self),
            Position(x, y),
            ...
        )
    end
end

local Goomba = LevelObject:extend("smb1_1.level_encoding.Goomba")
do
    binser.registerClass(Goomba)
    
    function Goomba:spawn(space)
        return game_objects.Goomba:new(space, self:position_get_coords())
    end
end

local Koopa = LevelObject:extend("smb1_1.level_encoding.Koopa")
do
    binser.registerClass(Koopa)
    
    function Koopa:spawn(space)
        return game_objects.Koopa:new(space, self:position_get_coords())
    end
end

local Player = LevelObject:extend("smb1_1.level_encoding.Player")
do
    binser.registerClass(Player)

    function Player:spawn(space)
        return game_objects.Player:new(space, self:position_get_coords())
    end
end

return {
    LevelObject = LevelObject,
    Goomba = Goomba,
    Koopa = Koopa,
    Player = Player,
}