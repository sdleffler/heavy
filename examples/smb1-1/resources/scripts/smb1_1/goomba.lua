local binser = require("std.binser")
local std_agent = require("std.agent")
local Agent, State = std_agent.Agent, std_agent.State
local gfx = hf.graphics
local GameObject = require("smb1_1.game_object").GameObject
local Velocity = hf.components.Velocity
local Collider = hf.components.Collider
local SpriteAnimation = hf.components.SpriteAnimation
local Player = require("smb1_1.player").Player

local dt = 1.0 / 60.0
local dying_time = 1
local walk_velocity = 2 * 16
local gravity_velocity = 8 * 16

local AliveState = State:extend("smb1_1.goomba.AliveState", { name = "alive" })
do
    function AliveState:init(agent, goomba) goomba:velocity_set_linear(-walk_velocity, 0) end

    function AliveState:update(agent, goomba)
        vx, _ = goomba:velocity_get_linear()
        goomba:velocity_set_linear(vx, -gravity_velocity)
    end

    function AliveState:on_squish(agent, goomba, player)
        if player then
            player:bounce(goomba)
        end
        agent:switch("dying", goomba)
        goomba:velocity_set_linear(0, 0)
    end

    function AliveState:on_collide_with_object(agent, goomba, object)
        vx, vy = goomba:velocity_get_linear()
        vx = -vx
        goomba:velocity_set_linear(vx, vy)
    end
end

local DyingState = State:extend("smb1_1.goomba.DyingState", { name = "dying" })
do
    function DyingState:init(agent, goomba)
        goomba.tag = rust.sprite_sheets.goomba:get_tag("dead")
        self.dying_counter = 0
        goomba:collider_remove()
    end

    function DyingState:update(agent, goomba)
        self.dying_counter = self.dying_counter + 1
        -- After 2 seconds, the goomba is officially dead
        if (self.dying_counter / 60) >= dying_time then goomba.to_despawn = true end
    end
end

local GoombaController = Agent:extend("GoombaController")
do
    GoombaController:add_states{ AliveState, DyingState }

    GoombaController:bind{ "update", "on_squish", "on_collide_with_object" }
end

local Goomba = GameObject:extend("smb1_1.game_objects.Goomba"):with(Velocity):with(Collider):with(
                   SpriteAnimation
               )
do
    binser.registerClass(Goomba)

    function Goomba:init(space, x, y)
        Goomba.super.init(
            self, space, x, y, Velocity(),
            SpriteAnimation(gfx.SpriteAnimation.new(rust.sprite_sheets.goomba)),
            Collider(hf.collision.Collider.cuboid(8, 8)), rust.GoombaMarker, rust.RequiresLuaUpdate,
            rust.Unloaded
        )
        self.tag = rust.sprite_sheets.goomba:get_tag("walk")
        self.last_tag = self.tag
        self.to_despawn = false
        self.controller = GoombaController:new()
        self:sprite_animation_goto_tag(self.tag)
    end

    function Goomba:update()
        self.controller:update(self, input)

        self:sprite_animation_update(dt)

        -- We only want to switch animations if the tag has changed; otherwise, we'll keep
        -- resetting the same animation over and over and it won't move, stuck at the starting
        -- frame.
        if self.tag ~= self.last_tag then
            self.last_tag = self.tag
            self:sprite_animation_goto_tag(self.tag)
        end

        -- Check if the goomba went out of bounds
        px, py = self:position_get_coords()
        if px < 0 or py < 0 then self.to_despawn = true end

        if self.to_despawn then rust.space:despawn(self) end
    end

    function Goomba:on_squish(player) self.controller:on_squish(self, player) end

    function Goomba:on_load() self.controller:push("alive", self) end

    function Goomba:on_collide_with_object(object)
        if not object:instanceOf(Player) then
            self.controller:on_collide_with_object(self, object)
        end
    end
end

return { Goomba = Goomba, GoombaController = GoombaController }
