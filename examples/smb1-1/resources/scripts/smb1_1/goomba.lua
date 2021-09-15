local std_agent = require("std.agent")
local Agent, State = std_agent.Agent, std_agent.State

local dying_counter = 0
local dt = 1.0/60.0
local dying_time = 2

local AliveState = State:extend("smb1_1.goomba.AliveState", { name = "alive" })
do
    function AliveState:update(agent, goomba)
        goomba:sprite_animation_update(dt)
        -- TODO: if stomp, then move to dying
    end
end

local DyingState = State:extend("smb1_1.goomba.DyingState", { name = "dying" })
do
    function DyingState:update(agent, goomba)
        dying_counter = dying_counter + 1
        -- After 2 seconds, the goomba is officially dead
        if (dying_counter / 60) >= dying_time then
            -- TODO Remove goomba from objects?
        end
    end
end

local GoombaController = Agent:extend("GoombaController")
do
    GoombaController:add_states {
        AliveState,
        DyingState,
    }

    GoombaController:bind {
        "update",
    }
end

return {
    GoombaController = GoombaController,
}