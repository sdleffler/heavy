local std_space = require("std.space")
local binser = require("std.binser")
local PlayerController = require("smb1_1.player").PlayerController
local Collider = hf.components.Collider
local Position = hf.components.Position
local Velocity = hf.components.Velocity
local SpriteAnimation = hf.components.SpriteAnimation
local gfx = hf.graphics

local game_objects = {}
do
    local GameObject = std_space.Object:extend("smb1_1.game_objects.GameObject")
        :with(Position)
    do
        game_objects.GameObject = GameObject
        binser.registerClass(GameObject)

        function GameObject:init(space, x, y, ...)
            GameObject.super.init(self, space, Position(x, y), ...)
        end
    end

    local Goomba = GameObject:extend("smb1_1.game_objects.Goomba")
    do
        game_objects.Goomba = Goomba
        binser.registerClass(Goomba)

        function Goomba:init(space, x, y)
            print("TODO: Goomba components")
            Goomba.super.init(self, space, x, y,
                Velocity(),
                SpriteAnimation(gfx.SpriteAnimation.new(rust.sprite_sheets.goomba)),
                rust.GoombaMarker
            )
        end
    end

    local Koopa = GameObject:extend("smb1_1.game_objects.Koopa")
    do
        game_objects.Koopa = Koopa
        binser.registerClass(Koopa)

        function Koopa:init(space, x, y)
            print("TODO: Koopa components")
            Koopa.super.init(self, space, x, y,
                Velocity(),
                SpriteAnimation(gfx.SpriteAnimation.new(rust.sprite_sheets.koopa)),
                rust.KoopaMarker
            )
        end
    end

    local Player = GameObject:extend("smb1_1.game_objects.Player")
        :with(Velocity)
        :with(Collider)
        :with(SpriteAnimation)
    do
        game_objects.Player = Player
        binser.registerClass(Player)

        function Player:init(space, x, y)
            Player.super.init(self, space, x, y,
                Velocity(),
                Collider(hf.collision.Collider.cuboid(8, 8)),
                SpriteAnimation(gfx.SpriteAnimation.new(rust.sprite_sheets.mario)),
                rust.PlayerMarker,
                rust.RequiresUpdate
            )
            self.run_frames = 0
            self.controller = PlayerController:new()
            self.controller:push("ground")
            self:sprite_animation_goto_tag_by_str("walk")
        end

        function Player:update()
            self.controller:update(self, input)
        end
    end
end

local level_objects = {}
do
    local LevelObject = std_space.Object:extend("smb1_1.level_objects.LevelObject")
        :with(Position)
    do
        level_objects.LevelObject = LevelObject
        binser.registerClass(LevelObject)
        
        function LevelObject:init(space, x, y, ...)
            LevelObject.super.init(self, space, Position(x, y), ...)
        end
    end

    local Goomba = LevelObject:extend("smb1_1.level_objects.Goomba")
    do
        level_objects.Goomba = Goomba
        binser.registerClass(Goomba)

        function Goomba:spawn(space)
            return game_objects.Goomba:new(space, self:position_get_coords())
        end
    end

    local Koopa = LevelObject:extend("smb1_1.level_objects.Koopa")
    do
        level_objects.Koopa = Koopa
        binser.registerClass(Koopa)

        function Koopa:spawn(space)
            return game_objects.Koopa:new(space, self:position_get_coords())
        end
    end

    local Player = LevelObject:extend("smb1_1.level_objects.Player")
    do
        level_objects.Player = Player
        binser.registerClass(Player)
        function Player:spawn(space)
            return game_objects.Player:new(space, self:position_get_coords())
        end
    end
end

return {
    level_objects = level_objects,
    game_objects = game_objects,
}