local binser = require("std.binser")
local std_agent = require("std.agent")
local Agent, State = std_agent.Agent, std_agent.State
local gfx = hf.graphics
local GameObject = require("smb1_1.game_object").GameObject
local item = require("smb1_1.item")
local Velocity = hf.components.Velocity
local Collider = hf.components.Collider
local SpriteAnimation = hf.components.SpriteAnimation

local button = game.button
local input = game.input_state

function sign(n) return (n > 0 and 1) or (n == 0 and 0) or -1 end

-- Mario-per-frame to pixels-per-frame.
function mpf_to_pps(pixels, subpixels, subsubpixels, subsubsubpixels)
    return 60 * (pixels + ((((subsubsubpixels / 16) + subsubpixels) / 16) + subpixels) / 16)
end

local min_walk_velocity = mpf_to_pps(0, 1, 3, 0)
local max_walk_velocity = mpf_to_pps(1, 9, 0, 0)
local max_run_velocity = mpf_to_pps(2, 9, 0, 0)
local walk_acceleration = mpf_to_pps(0, 0, 9, 8)
local run_acceleration = mpf_to_pps(0, 0, 14, 4)
local release_deceleration = mpf_to_pps(0, 0, 13, 0)
local skidding_deceleration = mpf_to_pps(0, 1, 10, 0)
local skid_turnaround_velocity = mpf_to_pps(0, 9, 0, 0)
local holding_a_gravity = mpf_to_pps(0, 2, 0, 0)
local normal_gravity = mpf_to_pps(0, 7, 0, 0)
local jump_impulse = mpf_to_pps(4, 0, 0, 0)
local maximum_falling_velocity = mpf_to_pps(4, 0, 0, 0)
local midair_low_acceleration = mpf_to_pps(0, 0, 9, 8)
local midair_low_deceleration = mpf_to_pps(0, 0, 14, 4)
local midair_high_acceleration = mpf_to_pps(0, 0, 14, 4)
local midair_high_deceleration = mpf_to_pps(0, 0, 13, 0)

local tag_dead = assert(game.sprite_sheets.mario:get_tag("dead"))
local tag_idle_smol = assert(game.sprite_sheets.mario:get_tag("idle"))
local tag_walk_smol = assert(game.sprite_sheets.mario:get_tag("walk"))
local tag_skid_smol = assert(game.sprite_sheets.mario:get_tag("skid"))
local tag_jump_smol = assert(game.sprite_sheets.mario:get_tag("jump"))
local tag_idle_beeg = assert(game.sprite_sheets.mario:get_tag("tall_idle"))
local tag_walk_beeg = assert(game.sprite_sheets.mario:get_tag("tall_walk"))
local tag_skid_beeg = assert(game.sprite_sheets.mario:get_tag("tall_skid"))
local tag_jump_beeg = assert(game.sprite_sheets.mario:get_tag("tall_jump"))
local tag_HENSHIN = assert(game.sprite_sheets.mario:get_tag("transform"))

local hurt_invincibility_len = 4 * 60

local GroundState = State:extend("smb1_1.player.GroundState", { name = "ground" })
do
    function GroundState:update(agent, player)
        if input:get_button_pressed(button.A) then
            player:velocity_add_linear(0, jump_impulse)

            -- TODO: beegsmol
            player.animation = tag_jump_smol
            agent:push("air")
        elseif not player.is_grounded then
            agent:push("air")
        else
            -- There are quite a few cases to consider here.
            -- 1.) Walking or running in the same direction as current velocity
            -- 2.) Walking or running in the opposite direction of current velocity
            -- 3.) Not walking or running but facing in the direction of current velocity (released)
            -- 4.) Not walking or running but facing in the opposite direction of current velocity
            -- (skidding)

            local left_down, right_down = input:get_button_down(button.Left),
                                          input:get_button_down(button.Right)

            if input:get_button_down(button.B) then
                player.run_frames = 10
            elseif left_down or right_down then
                player.run_frames = math.max(player.run_frames - 1, 0)
            else
                player.run_frames = 0
            end

            local running = player.run_frames > 0

            local vx, vy = player:velocity_get_linear()
            local sign_vx = sign(vx)
            local abs_vx = sign_vx * vx
            local move_dir = (left_down and -1 or 0) + (right_down and 1 or 0)

            if move_dir == 0 then
                -- Case 3. (released)
                if abs_vx > release_deceleration then
                    vx = vx - sign_vx * release_deceleration
                else
                    vx = 0
                end

                -- TODO: beegsmol
                player.animation = tag_idle_smol
            elseif move_dir == -sign_vx then
                -- Case 2 and 4. (skidding)
                if abs_vx > skid_turnaround_velocity + skidding_deceleration then
                    vx = vx - sign_vx * skidding_deceleration
                else
                    vx = 0
                end

                -- TODO: beegsmol
                player.animation = tag_skid_smol
            else
                assert(move_dir == sign_vx or sign_vx == 0)
                -- Case 1. (accelerating)
                local acceleration = (running and run_acceleration) or walk_acceleration
                local max_velocity = (running and max_run_velocity) or max_walk_velocity

                if abs_vx < min_walk_velocity then
                    -- If we're at less than minimum velocity, go to minimum velocity.
                    vx = move_dir * min_walk_velocity
                elseif abs_vx < max_velocity - acceleration then
                    -- If we're at less than max velocity and accelerating by `acceleration` won't
                    -- put us over the max velocity, then add it in.
                    vx = vx + move_dir * acceleration
                elseif abs_vx > max_velocity - release_deceleration then
                    -- If we're at more than max velocity and decelerating by `release_acceleration`
                    -- won't put us under the max velocity, then sub it out.
                    vx = vx - move_dir * release_deceleration
                else
                    -- If either accelerating or decelerating would put us at the max velocity, just
                    -- set it and forget it.
                    vx = move_dir * max_velocity
                end

                -- TODO: beegsmol
                if move_dir ~= 0 and sign_vx ~= 0 then
                    player.animation = tag_walk_smol
                else
                    player.animation = tag_idle_smol
                end
            end

            if sign_vx ~= 0 then player.facing_direction = sign_vx end

            player:sprite_animation_update(math.max(abs_vx, max_walk_velocity / 1.5) / 100 / 60)
            player:velocity_set_linear(vx, math.max(vy - normal_gravity, -maximum_falling_velocity))
        end
    end
end

local AirState = State:extend("smb1_1.player.AirState", { name = "air" })
do
    function AirState:update(agent, player)
        -- There are quite a few cases to consider here.
        -- 1.) Walking or running in the same direction as current velocity
        -- 2.) Walking or running in the opposite direction of current velocity
        -- 3.) Not walking or running but facing in the direction of current velocity (released)
        -- 4.) Not walking or running but facing in the opposite direction of current velocity
        -- (skidding)

        local left_down, right_down = input:get_button_down(button.Left),
                                      input:get_button_down(button.Right)

        local vx, vy = player:velocity_get_linear()
        local sign_vx = sign(vx)
        local abs_vx = sign_vx * vx
        local move_dir = (left_down and -1 or 0) + (right_down and 1 or 0)

        if move_dir == sign_vx or sign_vx == 0 then
            if abs_vx < max_walk_velocity then
                vx = vx + move_dir * midair_low_acceleration
            else
                vx = vx + move_dir * midair_high_acceleration
            end
        elseif move_dir == -sign_vx then
            if abs_vx < max_walk_velocity then
                vx = vx + move_dir * midair_low_deceleration
            else
                vx = vx + move_dir * midair_high_deceleration
            end
        end

        if input:get_button_down(button.A) then
            vy = vy - holding_a_gravity
        else
            vy = vy - normal_gravity
        end

        local max_velocity = (player.run_frames > 0 and max_run_velocity) or max_walk_velocity

        if vx > max_velocity then
            vx = max_velocity
        elseif vx < -max_velocity then
            vx = -max_velocity
        end

        player:velocity_set_linear(vx, math.max(vy, -maximum_falling_velocity))

        if player.is_grounded then agent:pop() end
    end
end

local Dead = State:extend("smb1_1.player.Dead", { name = "dead" })
do
    function Dead:init(agent, player)
        player.animation = tag_dead
        player:collider_remove()
        player:velocity_set_linear(0, 0)
        smb.controller:switch("player_died")
    end

    function Dead:update(agent, player)
        coroutine.resume(player.death_animation, player)
        if coroutine.status(player.death_animation) == "dead" then
            smb.controller:switch("wipe_out")
        end
    end
end

local PlayerController = Agent:extend("PlayerController")
do
    PlayerController:add_states{ GroundState, AirState, Dead }

    PlayerController:bind{ "update" }
end

local Player = GameObject:extend("smb1_1.game_objects.Player"):with(Velocity):with(Collider):with(
                   SpriteAnimation
               )
do
    binser.registerClass(Player)

    function Player:init(space, x, y)
        Player.super.init(
            self, space, x, y, Velocity(), Collider(hf.collision.Collider.cuboid(7.9, 8)),
            SpriteAnimation(gfx.SpriteAnimation.new(game.sprite_sheets.mario)), game.PlayerMarker,
            game.RequiresLuaUpdate
        )
        self.is_big = false
        self.run_frames = 0
        self.invincible_timer = 0
        self.facing_direction = 1
        self.animation = game.sprite_sheets.mario:get_tag("idle")
        self.prev_animation = self.animation
        self.controller = PlayerController:new()

        self.death_animation = coroutine.create(
                                   function(player)
                initial_pause = 0.5 * 60
                -- initial pause
                for i = 0, initial_pause, 1 do coroutine.yield() end
                player:velocity_set_linear(0, jump_impulse)
                coroutine.yield()
                local _, yp = player:position_get_coords()
                while (yp >= -8) do
                    _, vy = player:velocity_get_linear()
                    player:velocity_set_linear(
                        0, math.max(vy - holding_a_gravity, -maximum_falling_velocity)
                    )
                    coroutine.yield()
                    _, yp = player:position_get_coords()
                end
            end
                               )

        self.controller:push("ground")
        self:sprite_animation_goto_tag(self.animation)
    end

    function Player:on_headbutt_block(x, y, tile_id, hittable)
        -- Big ol' TODO.
        print(
            "BLOCK HEADBUTT! x: " .. tostring(x) .. ", y: " .. tostring(y) .. ", tile ID: " ..
                tostring(tile_id)
        )

        if hittable then
            game:set_tile(x, y, hittable, 0)
            local player_x, _ = self:position_get_coords()
            local direction
            if player_x <= (x + 0.5) * 16 then
                direction = 64
            else
                direction = -64
            end
            item.Mushroom:new(game.space, (x + 0.5) * 16, (y + 0.5) * 16, direction)
        elseif tile_id == 241 then
            game:remove_tile(x, y)
        end
    end

    -- Called when we successfully bounce on an enemy (squish a goomba or such.)
    function Player:bounce(enemy)
        local x, _ = self:velocity_get_linear()
        self:velocity_set_linear(x, jump_impulse)
    end

    function Player:on_collide_with_object(object)
        local _, y = self:velocity_get_linear()

        -- If we are moving downwards (yes, this is how the OG SMB1 did it too) then count as
        -- SQUEESH
        if y < 0 then
            if object.on_squish then object:on_squish(self) end
        elseif object.on_mario_collide then
            object:on_mario_collide(self)
        else
            self:hurt(object)
        end
        -- TODO: add case for coins?
    end

    function Player:hurt(enemy)
        if self.invincible_timer == 0 then
            if not self.is_big then self.controller:switch("dead", self) end
            self.invincible_timer = hurt_invincibility_len
        end
    end

    function Player:update()
        self.controller:update(self, input)

        -- if mario is currently invincible (either due to a star or due to an enemy hitting him),
        -- subtract the timer until he is no longer invincible
        if self.invincible_timer > 0 then self.invincible_timer = self.invincible_timer - 1 end

        -- We only want to switch animations if the tag has changed; otherwise, we'll keep
        -- resetting the same animation over and over and it won't move, stuck at the starting
        -- frame.
        if self.animation ~= self.prev_animation then
            self.prev_animation = self.animation
            self:sprite_animation_goto_tag(self.animation)
        end
    end
end

return { Player = Player, PlayerController = PlayerController }
