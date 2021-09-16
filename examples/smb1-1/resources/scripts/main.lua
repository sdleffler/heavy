-- Grab the global `space` and shove it into a local variable.
is_player_dead = false
local space = rust.space

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

function hv.update()
    -- TODO
end

function hv.draw()
    -- TODO (will this even be done in Lua at all?)
end
