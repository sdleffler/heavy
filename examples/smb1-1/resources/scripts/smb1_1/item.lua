local binser = require("std.binser")
local std_agent = require("std.agent")
local Agent, State = std_agent.Agent, std_agent.State
local gfx = hf.graphics
local GameObject = require("smb1_1.game_object").GameObject
local Velocity = hf.components.Velocity
local Collider = hf.components.Collider

local emerge_time = 1
local tile_size = 16

local EmergingState = State:extend("smb1_1.item.EmergingState", { name = "emerging" })
do
    function EmergingState:init(agent, item, next_state)
        self.t = 0
        self.x, self.y = item:position_get_coords()
        self.next_state = next_state
    end

    function EmergingState:update(agent, item, dt)
        self.t = self.t + dt
        if self.t > emerge_time then
            item:position_set_coords(self.x, self.y + tile_size)
            agent:switch(self.next_state, item)
        else
            item:position_set_coords(self.x, self.y + (self.t / emerge_time) * tile_size)
        end
    end
end

local ItemController = Agent:extend("smb1_1.item.ItemController")
do
    ItemController:add_states{ EmergingState }
    ItemController:bind{ "update" }
end

local MushroomController = ItemController:extend("smb1-1.item.MushroomController")
do
    local MushroomState = State:extend("smb1_1.item.MushroomState", { name = "mushroom" })
    do
        function MushroomState:init(agent, item)
            game.space:insert(
                item, Velocity(item.direction, 0), Collider(hf.collision.Collider.cuboid(8, 8))
            )
        end

        function MushroomState:update(agent, item)
            local vx, _ = item:velocity_get_linear()
            item:velocity_set_linear(vx, -128)
        end
    end

    MushroomController:add_states{ MushroomState }
end

local Mushroom = GameObject:extend("smb1_1.item.Mushroom"):with(Velocity):with(Collider)
do
    binser.registerClass(Mushroom)

    function Mushroom:init(space, x, y, direction)
        Mushroom.super.init(self, space, x, y, game.RequiresLuaUpdate, game.ItemMarker(4))
        self.direction = direction
        self.controller = MushroomController:new()
        self.controller:push("emerging", self, "mushroom")
        self.to_despawn = false
    end

    function Mushroom:on_mario_collide(player, player_tag_table)
        if not (player.powerup_status == "big" or player.powerup_status == "fireflower") then
            player.powerup_status = "big"
            if not player.is_grounded then
                player:sprite_animation_goto_tag_by_str("tall_jump")
            end
        end

        self.to_despawn = true
    end

    function Mushroom:update(dt)
        self.controller:update(self, dt)
        if self.to_despawn then game.space:despawn(self) end
    end
end

return { Mushroom = Mushroom }
