-- The special "game" object contains a Lua userdata referring to the instance of `SmbOneOne`.
is_player_dead = false
local space = game.space

function load_level()
    space:clear()

    local level = load("smb1_1.level")()
    local all_objects = level:objects()

    for _, object in ipairs(all_objects) do
        print("object: " .. tostring(object))
        object:spawn(space)
    end
end

function hv.load() load_level() end

function hv.update(dt)
    -- Delegate to the full game update method on the game object.
    game:update(dt)
end

function hv.draw()
    -- Delegate to the game draw method on the game object.
    game:draw()
end
