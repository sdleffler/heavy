local binser = require("std.binser")
local std_agent = require("std.agent")
local Agent, State = std_agent.Agent, std_agent.State
local gfx = hf.graphics
local GameObject = require("smb1_1.game_object").GameObject
local Velocity = hf.components.Velocity
local Collider = hf.components.Collider
local SpriteAnimation = hf.components.SpriteAnimation
local Player = require("smb1_1.player").Player

local revival_length = 10.0
local almost_reviving_length = 8.0

-- Need to set these once they actually move
local shelled_velocity = 10 * 16
local walk_velocity = 2 * 16
local gravity_velocity = 8 * 16
local shell_speed = 0
local dt = 1.0 / 60.0

local tag_walk = assert(rust.sprite_sheets.koopa:get_tag("walk"))
local tag_in_shell = assert(rust.sprite_sheets.koopa:get_tag("shell_spin"))
local tag_reviving = assert(rust.sprite_sheets.koopa:get_tag("reviving"))

function launch_shell(player, koopa)
    pp_x, _ = player:position_get_coords()
    kp_x, _ = koopa:position_get_coords()
    vx, vy = koopa:velocity_get_linear()
    if pp_x >= kp_x then
        koopa:velocity_set_linear(-shelled_velocity, vy)
    else
        koopa:velocity_set_linear(shelled_velocity, vy)
    end
end

local Walking = State:extend("smb1_1.koop.walking", { name = "walk" })
do
    function Walking:init(agent, koopa)
        koopa:velocity_set_linear(-walk_velocity, 0)
        koopa.tag = tag_walk
    end

    function Walking:update(agent, koopa)
        vx, _ = koopa:velocity_get_linear()
        koopa:velocity_set_linear(vx, -gravity_velocity)
        koopa:sprite_animation_update(1.0 / 60.0)
    end

    function Walking:on_squish(agent, koopa, player)
        player:bounce(koopa)
        agent:switch("shell_stop", koopa)
    end

    function Walking:on_mario_collide(agent, koopa, player) player:hurt(koopa) end

    function Walking:on_enemy_collide(agent, koopa, enemy)
        vx, vy = goomba:velocity_get_linear()
        vx = -vx
        koopa:velocity_set_linear(vx, vy)
    end
end

local ShellStop = State:extend("smb1_1.koopa.ShellStop", { name = "shell_stop" })
do
    function ShellStop:init(agent, koopa)
        koopa.tag = tag_in_shell
        _, vy = koopa:velocity_get_linear()
        koopa:velocity_set_linear(0, vy)
        self.revive_timer = 0
    end

    function ShellStop:update(agent, koopa)
        self.revive_timer = self.revive_timer + 1

        -- if the koopa is about to revive, start swaping the animations
        if (self.revive_timer / 60) >= almost_reviving_length then
            if (self.revive_timer % 3) == 0 then
                if koopa.tag == tag_reviving then
                    koopa.tag = tag_in_shell
                else
                    koopa.tag = tag_reviving
                end
            end
        end

        if (self.revive_timer / 60) >= revival_length then agent:switch("walk", koopa) end

        -- TODO: need to check for collision and enter shell drift state
    end

    function ShellStop:on_squish(agent, koopa, player)
        player:bounce(koopa)
        agent:switch("shell_drift", koopa, player)
    end

    function ShellStop:on_mario_collide(agent, koopa, player)
        agent:switch("shell_drift", koopa, player)
    end

    function ShellStop:on_enemy_collide(agent, koopa, enemy)
        vx, vy = koopa:velocity_get_linear()
        vx = -vx
        koopa:velocity_set_linear(vx, vy)
    end
end

local ShellDrift = State:extend("smb1_1.koopa.ShellDrift", { name = "shell_drift" })
do
    function ShellDrift:init(agent, koopa, player) launch_shell(player, koopa) end

    function ShellDrift:update(agent, koopa)
        vx, _ = koopa:velocity_get_linear()
        koopa:velocity_set_linear(vx, -gravity_velocity)
    end

    function ShellDrift:on_squish(agent, koopa, player)
        player:bounce(koopa)
        agent:switch("shell_stop", koopa)
    end

    function ShellDrift:on_mario_collide(agent, koopa, player) player:hurt(koopa) end

    function ShellDrift:on_enemy_collide(agent, koopa, enemy) enemy:on_squish() end
end

local KoopaController = Agent:extend("KoopaController")
do
    KoopaController:add_states{ Walking, ShellStop, ShellDrift }
    KoopaController:bind{
        "update", "on_squish", "on_mario_collide", "on_collide_with_object", "on_mario_collide",
        "on_enemy_collide",
    }
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
            rust.RequiresLuaUpdate, rust.Unloaded
        )
        self.tag = rust.sprite_sheets.koopa:get_tag("walk")
        self.to_despawn = false;
        self.last_tag = self.tag
        self.controller = KoopaController:new()
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

        -- Check if the koopa needs to be despawned
        px, py = self:position_get_coords()
        if px < 0 or py < 0 then self.to_despawn = true end

        if self.to_despawn then rust.space:despawn(self) end
    end

    function Koopa:on_squish(player) self.controller:on_squish(self, player) end

    function Koopa:on_collide_with_object(object)
        if not object:instanceOf(Player) then
            -- TODO: collisions with breakable blocks/blocks with items in them?
            self.controller:on_enemy_collide(self, object)
        end
    end

    function Koopa:on_mario_collide(player)
        self.controller:on_mario_collide(self, player)
    end

    function Koopa:on_load() self.controller:push("walk", self) end
end

return { Koopa = Koopa, KoopaController = KoopaController }
