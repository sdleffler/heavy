local std_agent = require("std.agent")
local Agent, State = std_agent.Agent, std_agent.State

-- The special "game" object contains a Lua userdata referring to the instance of `SmbOneOne`.
local space = game.space
smb = {}

local Normal = State:extend("main.Normal", { name = "normal" })
do
    function Normal:init(agent)
        space:clear()

        local level = load("smb1_1.level")()
        local all_objects = level:objects()

        for _, object in ipairs(all_objects) do
            print("object: " .. tostring(object))
            object:spawn(space)
        end
    end

    function Normal:update(agent, dt) game:update_normal(dt) end
end

local PlayerDied = State:extend("main.PlayerDied", { name = "player_died" })
do function PlayerDied:update(agent, dt) game:update_player_died(dt) end end

local Resetting = State:extend("main.Resetting", { name = "resetting" })
do
    function Resetting:init(agent)
        print("RESETTING YO")
        agent:switch("normal")
    end
    function Resetting:update(agent, dt)
        -- TODO: fill this function out
    end
end

local GameController = Agent:extend("GameController")
do
    GameController:add_states{ Normal, PlayerDied, Resetting }

    GameController:bind{ "update" }
end

function hv.load()
    smb.controller = GameController:new()
    smb.controller:push("normal")
end

function hv.update(dt)
    smb.controller:update(dt)
    game:update_object_sprite_batches()
end

function hv.draw()
    -- Delegate to the game draw method on the game object.
    game:draw()
end
