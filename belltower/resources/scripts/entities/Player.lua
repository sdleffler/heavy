local agent = require("std.agent")
local hf = require("hf")

local Isometry2 = hf.math.Isometry2

local Player = agent.Agent:extend("Player", {})
    :with(hf.components.Position)
    :with(hf.components.Velocity)

function Player:init(space)
    space:spawn(
        hv.components.ObjectTable(self),
        hv.components.UpdateHook(),
        hv.components.game.PlayerController(),
        hv.components.game.CombatGeometry(),
        hf.components.Position(120, 120),
        hf.components.Velocity(120, 120)
    )
end

-- local frame = 0
function Player:update()
    -- if frame == 0 then
    --     hv.logger.info("player", "hello!!")
    -- end
    -- frame = (frame + 1) % 60
end

return Player
