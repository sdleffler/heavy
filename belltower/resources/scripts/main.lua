local Player = require("entities.Player")

local player = Player:new(space)

hv.logger.info("main", tostring(player))

function hv.update()
    -- hv.logger.info("update", "hello")
end

-- function hv.draw()
--     -- hv.logger.info("draw", "hello")
-- end
