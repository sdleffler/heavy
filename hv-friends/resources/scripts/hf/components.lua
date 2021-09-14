local hf_math = require("hf.math")

-- Position mixin, for use w/ std.class.
local Position = {}
do
    local hf_position = hv.plugins.friends.position
    local tmp = hf_math.Position2.identity()

    local hf_create_position_constructor = hf_position.create_position_constructor
    setmetatable(Position, {
        __call = function(_, ...)
            if select(1, ...) ~= nil and type(select(1, ...)) == "userdata" then
                return hf_create_position_constructor(select(1, ...))
            else
                local x, y, angle = ...
                tmp:init(x or 0, y or 0, angle or 0)
                return hf_create_position_constructor(tmp)
            end
        end
    })

    local has_position = hf_position.has_position
    function Position:has_position()
        return has_position(self)
    end

    local get_position2 = hf_position.get_position2
    function Position:position_get(out)
        local out = out or tmp:clone()
        get_position2(self, out)
        return out
    end

    function Position:position_get_coords()
        get_position2(self, tmp)
        return tmp.x, tmp.y
    end

    function Position:position_get_angle()
        get_position2(self, tmp)
        return tmp.angle
    end

    local set_position2 = hf_position.set_position2
    function Position:position_set(...)
        if type(select(1, ...)) == "userdata" then
            set_position2(self, select(1, ...))
        else
            tmp:init(...)
            set_position2(self, tmp)
        end
    end

    function Position:position_set_coords(x, y)
        get_position2(self, tmp)
        tmp:set_coords(x, y)
        set_position2(self, tmp)
    end

    function Position:position_set_angle(angle)
        get_position2(self, tmp)
        tmp:set_angle(angle)
        set_position2(self, tmp)
    end

    function Position:position_add_angle(angle)
        get_position2(self, tmp)
        tmp:add_angle(angle)
        set_position2(self, tmp)
    end
end

local Velocity = {}
do
    local hf_velocity = hv.plugins.friends.velocity
    local tmp = hf_math.Velocity2.zero()

    local hf_create_velocity_constructor = hf_velocity.create_velocity_constructor
    setmetatable(Velocity, {
        __call = function(_, ...)
            if select(1, ...) ~= nil and type(select(1, ...)) == "userdata" then
                return hf_create_velocity_constructor(select(1, ...))
            else
                local x, y, angular = ...
                tmp:init(x or 0, y or 0, angular or 0)
                return hf_create_velocity_constructor(tmp)
            end
        end
    })
    
    local has_velocity = hf_velocity.has_velocity
    function Velocity:has_velocity()
        return has_velocity(self)
    end
    
    local get_velocity2 = hf_velocity.get_velocity2
    function Velocity:velocity(out)
        local out = out or tmp:clone()
        get_velocity2(self, out)
        return out
    end

    local set_velocity2 = hf_velocity.set_velocity2
    function Velocity:velocity_set(...)
        if type(select(1, ...)) == "userdata" then
            set_velocity2(self, select(1, ...))
        else
            tmp:init(...)
            set_velocity2(self, tmp)
        end
    end

    function Velocity:velocity_set_linear(x, y)
        get_velocity2(self, tmp)
        tmp:set_linear(x, y)
        set_velocity2(self, tmp)
    end

    function Velocity:velocity_add_linear(x, y)
        get_velocity2(self, tmp)
        tmp:add_linear(x, y)
        set_velocity2(self, tmp)
    end

    function Velocity:velocity_get_linear()
        get_velocity2(self, tmp)
        return tmp.x, tmp.y
    end

    function Velocity:velocity_set_angular(angular)
        get_velocity2(self, tmp)
        tmp:set_angular(angular)
        set_velocity2(self, tmp)
    end
    
    function Velocity:velocity_get_angular()
        get_velocity2(self, tmp)
        return tmp.angular
    end
end

local Collider = {}
do
    local hf_collision = hv.plugins.friends.collision

    local hf_create_collider_constructor = hf_collision.create_collider_component
    setmetatable(Collider, {
        __call = function(_, collider)
            return hf_create_collider_constructor(collider)
        end,
    })

    -- Temporary collider userdata to be overwritten by fetch/set functions.
    local tmp = hf_collision.create_ball(0.)
    
    local hf_get_collider = hf_collision.get_collider
    function Collider:collider(out)
        local out = out or tmp:clone()
        hf_get_collider(self, out)
        return out
    end

    Collider.collider_set = hf_collision.set_collider

    local tmp_aabb = hf_math.Box2.invalid()
    function Collider:collider_compute_local_aabb(out)
        hf_get_collider(self, tmp)
        out = out or tmp_aabb:clone()
        tmp:compute_local_aabb(out)
        return out
    end

    function Collider:collider_compute_aabb(tx, out)
        hf_get_collider(self, tmp)
        out = out or tmp_aabb:clone()
        tmp:compute_aabb(tx, out)
        return out
    end
    
    function Collider:collider_compute_swept_aabb(start_tx, end_tx, out)
        hf_get_collider(self, tmp)
        out = out or tmp_aabb:clone()
        tmp:compute_swept_aabb(start_tx, end_tx, out)
        return out
    end
end

local SpriteAnimation = {}
do
    local hf_sprite = hv.plugins.friends.graphics.sprite
    local hf_create_sprite_animation_cc =
        hf_sprite.create_sprite_animation_component_constructor
    local tmp = hf_sprite.dummy_sheet:clone()

    setmetatable(SpriteAnimation, {
        __call = function(_, sprite_animation)
            return hf_create_sprite_animation_cc(sprite_animation)
        end,
    })

    local hf_get_sprite_animation = hf_sprite.get_sprite_animation
    function SpriteAnimation:sprite_animation_get(out)
        out = out or tmp:clone()
        hf_get_sprite_animation(self, out)
        return out
    end
    
    local hf_set_sprite_animation = hf_sprite.set_sprite_animation
    function SpriteAnimation:sprite_animation_set(animation)
        hf_set_sprite_animation(self, animation)
    end

    function SpriteAnimation:sprite_animation_update(dt)
        hf_get_sprite_animation(self, tmp)
        tmp:update(dt)
        hf_set_sprite_animation(self, tmp)
    end

    function SpriteAnimation:sprite_animation_set_paused(paused)
        hf_get_sprite_animation(self, tmp)
        tmp:set_paused(paused)
        hf_set_sprite_animation(self, tmp)
    end

    function SpriteAnimation:sprite_animation_is_paused()
        hf_get_sprite_animation(self, tmp)
        return tmp:is_paused()
    end

    function SpriteAnimation:sprite_animation_set_loop(should_loop)
        hf_get_sprite_animation(self, tmp)
        tmp:set_loop(should_loop)
        hf_set_sprite_animation(self, tmp)
    end

    function SpriteAnimation:sprite_animation_should_loop()
        hf_get_sprite_animation(self, tmp)
        return tmp:should_loop()
    end
    
    function SpriteAnimation:sprite_animation_goto_tag(tag)
        hf_get_sprite_animation(self, tmp)
        tmp:goto_tag(tag)
        hf_set_sprite_animation(self, tmp)
    end
    
    function SpriteAnimation:sprite_animation_goto_tag_by_str(tag_name)
        hf_get_sprite_animation(self, tmp)
        tmp:goto_tag_by_str(tag_name)
        hf_set_sprite_animation(self, tmp)
    end
end

return {
    Collider = Collider,
    Position = Position,
    Velocity = Velocity,
    SpriteAnimation = SpriteAnimation,
}