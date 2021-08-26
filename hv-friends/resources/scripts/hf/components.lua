local hf_math = require("hf.math")

-- Position mixin, for use w/ std.class.
local Position = {}
do
    local hf_position = hv.plugins.friends.position
    local tmp = hf_math.Isometry2.identity()

    local hf_create_position_constructor = hf_position.create_position_constructor
    setmetatable(Position, {
        __call = function(_, ...)
            if type(select(1, ...)) == "userdata" then
                return hf_create_position_constructor(select(1, ...))
            else
                local x, y, angle = ...
                tmp:init(x or 0, y or 0, angle or 0)
                return hf_create_position_constructor(tmp)
            end
        end
    })

    local get_isometry2 = hf_position.get_isometry2
    function Position:position(out)
        local out = out or tmp:clone()
        get_isometry2(self, out)
        return out
    end

    local set_isometry2 = hf_position.set_isometry2
    function Position:set_position(...)
        if type(select(1, ...)) == "userdata" then
            set_isometry2(self, select(1, ...))
        else
            tmp:init(...)
            set_isometry2(self, tmp)
        end
    end

    function Position:set_translation(x, y)
        get_isometry2(self, tmp)
        tmp:set_translation(x, y)
        set_isometry2(self, tmp)
    end

    function Position:set_rotation(angle)
        get_isometry2(self, tmp)
        tmp:set_rotation(angle)
        set_isometry2(self, tmp)
    end
end

local Velocity = {}
do
    local hf_velocity = hv.plugins.friends.velocity
    local tmp = hf_math.Velocity2.zero()

    local hf_create_velocity_constructor = hf_velocity.create_velocity_constructor
    setmetatable(Velocity, {
        __call = function(_, ...)
            if type(select(1, ...)) == "userdata" then
                return hf_create_velocity_constructor(select(1, ...))
            else
                local x, y, angular = ...
                tmp:init(x or 0, y or 0, angular or 0)
                return hf_create_velocity_constructor(tmp)
            end
        end
    })
    
    local get_velocity2 = hf_velocity.get_velocity2
    function Velocity:velocity(out)
        local out = out or tmp:clone()
        get_velocity2(self, out)
        return out
    end

    local set_velocity2 = hf_velocity.set_velocity2
    function Velocity:set_velocity(...)
        if type(select(1, ...)) == "userdata" then
            set_velocity2(self, select(1, ...))
        else
            tmp:init(...)
            set_velocity2(self, tmp)
        end
    end

    function Velocity:set_linear_velocity(x, y)
        get_velocity2(self, tmp)
        tmp:set_linear(x, y)
        set_velocity2(self, tmp)
    end

    function Velocity:set_angular_velocity(angular)
        get_velocity2(self, tmp)
        tmp:set_angular(angular)
        set_velocity2(self, tmp)
    end
end

local SpriteAnimationState = {}
do
    local hf_create_sprite_animation_state_constructor =
        hv.plugins.friends.graphics.hf_create_sprite_animation_state_component_constructor

    setmetatable(SpriteAnimationState, {
        __call = function(_, sprite_sheet, tag, should_loop)
            return hf_create_sprite_animation_state_constructor(sprite_sheet, tag, should_loop)
        end,
    })
end

return {
    Position = Position,
    Velocity = Velocity,
}