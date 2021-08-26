local hv_danmaku = hv.plugins.danmaku

local state_registry = hv_danmaku.get_state_registry()

local bind = function(name)
    local f = state_registry[name]
    return function(...)
        return f(state_registry, ...)
    end
end

return {
    lerp_linear_speed = bind("lerp_linear_speed"),
    lerp_polar_angular_speed = bind("lerp_polar_angular_speed"),
    lerp_polar_linear_speed = bind("lerp_polar_linear_speed"),
    kill = bind("kill"),
    parallel = bind("parallel"),
    sequence = bind("sequence"),
    sprite = bind("sprite"),
    sprite_sequence = bind("sprite_sequence"),
    wait = bind("wait"),
    nil
}