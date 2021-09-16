local binser = require("std.binser")
local std_agent = require("std.agent")
local Agent, State = std_agent.Agent, std_agent.State
local gfx = hf.graphics
local GameObject = require("smb1_1.game_object").GameObject
local Velocity = hf.components.Velocity
local Collider = hf.components.Collider
local SpriteAnimation = hf.components.SpriteAnimation

local revival_length = 10.0
local almost_reviving_length = 8.0

-- Need to set these once they actually move
local walk_speed = 0
local shell_speed = 0
local dt = 1.0 / 60.0

local tag_walk = assert(rust.sprite_sheets.koopa:get_tag("walk"))
local tag_in_shell = assert(rust.sprite_sheets.koopa:get_tag("shell_spin"))
local tag_reviving = assert(rust.sprite_sheets.koopa:get_tag("reviving"))

local Walking = State:extend("smb1_1.koop.walking", { name = "walk" })
do
    function Walking:update(agent, koopa)
        koopa:sprite_animation_update(1.0 / 60.0)
        -- TODO: implement walking
    end
    function Walking:on_squish(agent, koopa)
        koopa.tag = tag_in_shell
        agent:push("shell_stop")
    end
end

local ShellStop = State:extend("smb1_1.koopa.ShellStop", { name = "shell_stop" })
do
    function ShellStop:update(agent, koopa)
        koopa.revive_timer = koopa.revive_timer + 1

        -- if the koopa is about to revive, start swaping the animations
        if (koopa.revive_timer / 60) >= almost_reviving_length then
            if (koopa.revive_timer % 3) == 0 then
                if koopa.tag == tag_reviving then
                    koopa.tag = tag_in_shell
                else
                    koopa.tag = tag_reviving
                end
            end
        end

        if (koopa.revive_timer / 60) >= revival_length then
            agent:pop()
            koopa.tag = tag_walk
            koopa.revive_timer = 0.0
        end

        -- TODO: need to check for collision and enter shell drift state
    end
    function ShellStop:on_squish(agent, koopa) agent:push("shell_drift") end
end

local ShellDrift = State:extend("smb1_1.koopa.ShellDrift", { name = "shell_drift" })
do
    function ShellDrift:update(agent, koopa)
        -- Implement the shell sliding
        -- Implement going back to shell stop after being bounced on
    end
    function ShellDrift:on_squish(agent, koopa) agent:pop() end
end

local KoopaController = Agent:extend("KoopaController")
do
    KoopaController:add_states{ Walking, ShellStop, ShellDrift }
    KoopaController:bind{ "update", "on_squish" }
end

local Koopa = GameObject:extend("smb1_1.game_objects.Koopa"):with(Velocity):with(Collider):with(
                  SpriteAnimation
              )
do
    binser.registerClass(Koopa)

    function Koopa:init(space, x, y)
        Koopa.super.init(
            self, space, x, y, Velocity(), Collider(hf.collision.Collider.cuboid(8, 8)),
            SpriteAnimation(gfx.SpriteAnimation.new(rust.sprite_sheets.koopa)), rust.KoopaMarker,
            rust.RequiresLuaUpdate
        )
        self.tag = rust.sprite_sheets.koopa:get_tag("walk")
        self.last_tag = self.tag
        self.controller = KoopaController:new()
        self.controller:push("walk")
        self.dead = false
        self.revive_timer = 0.0
        self:sprite_animation_goto_tag(self.tag)
    end

    function Koopa:update()
        self.controller:update(self, input)

        -- We only want to switch animations if the tag has changed; otherwise, we'll keep
        -- resetting the same animation over and over and it won't move, stuck at the starting
        -- frame.
        if self.tag ~= self.last_tag then
            self.last_tag = self.tag
            self:sprite_animation_goto_tag(self.tag)
        end
    end

    function Koopa:on_squish(player) self.controller:on_squish(self, input) end
end

return { Koopa = Koopa, KoopaController = KoopaController }
