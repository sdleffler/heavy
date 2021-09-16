local binser = require("std.binser")
local std_agent = require("std.agent")
local Agent, State = std_agent.Agent, std_agent.State
local gfx = hf.graphics
local GameObject = require("smb1_1.game_object").GameObject
local Velocity = hf.components.Velocity
local Collider = hf.components.Collider
local SpriteAnimation = hf.components.SpriteAnimation

local dt = 1.0 / 60.0
local dying_time = 1

local AliveState = State:extend("smb1_1.goomba.AliveState", { name = "alive" })
do
    function AliveState:on_squish(agent, goomba, player)
        player:bounce(goomba)
        agent:switch("dying", goomba)
    end
end

local DyingState = State:extend("smb1_1.goomba.DyingState", { name = "dying" })
do
    function DyingState:init(agent, goomba)
        goomba.tag = rust.sprite_sheets.goomba:get_tag("dead")
        self.dying_counter = 0
    end

    function DyingState:update(agent, goomba)
        self.dying_counter = self.dying_counter + 1
        -- After 2 seconds, the goomba is officially dead
        if (self.dying_counter / 60) >= dying_time then rust.space:despawn(goomba) end
    end
end

local GoombaController = Agent:extend("GoombaController")
do
    GoombaController:add_states{ AliveState, DyingState }

    GoombaController:bind{ "update", "on_squish" }
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
            Collider(hf.collision.Collider.cuboid(8, 8)), rust.GoombaMarker, rust.RequiresLuaUpdate
        )
        self.tag = rust.sprite_sheets.goomba:get_tag("walk")
        self.last_tag = self.tag
        self.controller = GoombaController:new()
        self.controller:push("alive")
        self:sprite_animation_goto_tag(self.tag)
    end

    function Goomba:update()
        self:sprite_animation_update(dt)
        self.controller:update(self, input)

        -- We only want to switch animations if the tag has changed; otherwise, we'll keep
        -- resetting the same animation over and over and it won't move, stuck at the starting
        -- frame.
        if self.tag ~= self.last_tag then
            self.last_tag = self.tag
            self:sprite_animation_goto_tag(self.tag)
        end
    end

    function Goomba:on_squish(player) self.controller:on_squish(self, player) end
end

return { Goomba = Goomba, GoombaController = GoombaController }
