local std_agent = require("std.agent")
local Agent, State = std_agent.Agent, std_agent.State
local gfx = hf.graphics

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

        -- TODO: handle loading a map from the Lua side rather than the Rust side, such that we
        -- don't hard-code the map into the Rust side.
        game:reset_map()

        agent:push("wipe_in")
    end

    function Normal:update(agent, dt)
        game:integrate_objects_without_colliders(dt)
        game:update_normal(dt)
    end

    function Normal:draw() game:draw() end
end

local PlayerDied = State:extend("main.PlayerDied", { name = "player_died" })
do
    function PlayerDied:update(agent, dt)
        game:integrate_objects_without_colliders(dt)
        game:update_player_died(dt)
    end
    function PlayerDied:draw() game:draw() end
end

local WIPE_TIME = 1.3
local NUM_WIPE_RECTS = 15

function inout_cubic(x)
    if x < 0.5 then
        return 4 * x * x * x
    else
        return 1 - ((-2 * x + 2) ^ 3) / 2
    end
end

local WipeIn = State:extend("main.WipeIn", { name = "wipe_in" })
do
    function WipeIn:init(agent) self.t = 0 end

    function WipeIn:update(agent, dt)
        self.t = self.t + dt
        if self.t > WIPE_TIME then agent:pop() end
    end

    function WipeIn:draw()
        -- We want to draw the wipe-in animation over the game.
        game:draw()

        local normalized_t = (self.t / WIPE_TIME) * (4 / 3)
        local width_x, width_y = gfx.get_dimensions()

        gfx.origin()
        gfx.set_color(0, 0, 0, 1)
        for i = 0, NUM_WIPE_RECTS - 1 do
            local offset_t = normalized_t + (i - NUM_WIPE_RECTS / 2) * (1 / 3) / NUM_WIPE_RECTS

            local x = -width_x * inout_cubic(offset_t)
            local y = i * (width_y / NUM_WIPE_RECTS)
            gfx.rectangle(gfx.DrawMode.Fill, x - 32, y, width_x + 32, width_y / NUM_WIPE_RECTS)
        end
    end
end

local WipeOut = State:extend("main.WipeOut", { name = "wipe_out" })
do
    function WipeOut:init(agent) self.t = 0 end

    function WipeOut:update(agent, dt)
        self.t = self.t + dt
        if self.t > WIPE_TIME then agent:switch("normal") end
    end

    function WipeOut:draw()
        -- We want to draw the wipe-in animation over the game.
        game:draw()

        local normalized_t = (self.t / WIPE_TIME) * (4 / 3)
        local width_x, width_y = gfx.get_dimensions()

        gfx.origin()
        gfx.set_color(0, 0, 0, 1)
        for i = 0, NUM_WIPE_RECTS - 1 do
            local offset_t = normalized_t + (i - NUM_WIPE_RECTS / 2) * (1 / 3) / NUM_WIPE_RECTS

            local x = width_x - width_x * inout_cubic(offset_t)
            local y = i * (width_y / NUM_WIPE_RECTS)
            gfx.rectangle(gfx.DrawMode.Fill, x, y, width_x * 2, width_y / NUM_WIPE_RECTS)
        end
    end
end

local GameController = Agent:extend("GameController")
do
    GameController:add_states{ Normal, PlayerDied, WipeIn, WipeOut }

    GameController:bind{ "update", "draw" }
end

function hv.load()
    smb.controller = GameController:new()
    smb.controller:push("normal")
end

function hv.update(dt)
    smb.controller:update(dt)
    game:update_object_sprite_batches()
end

function hv.draw() smb.controller:draw() end
