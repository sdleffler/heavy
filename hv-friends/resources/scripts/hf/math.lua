local hf_math = hv.plugins.friends.math

local Position2 = {}
do
    Position2.new = hf_math.create_position2_object
    Position2.identity = hf_math.create_position2_object_from_identity
    
    function Position2.from_translation(x, y)
        return Position2.new(x, y, 0)
    end

    function Position2.from_rotation(angle)
        return Position2.new(0, 0, angle)
    end

    setmetatable(Position2, {
        __call = function(_, x, y, angle)
            return Position2.new(x or 0, y or 0, angle or 0)
        end 
    })
end

local Velocity2 = {}
do
    Velocity2.new = hf_math.create_velocity2_object
    Velocity2.zero = hf_math.create_velocity2_object_from_zero
    
    function Velocity2.from_linear(x, y)
        return Velocity2.new(x, y, 0)
    end

    function Velocity2.from_angular(angular)
        return Velocity2.new(0, 0, angular)
    end

    setmetatable(Velocity2, {
        __call = function(_, x, y, angular)
            return Velocity2.new(x or 0, y or 0, angular or 0)
        end 
    })
end

local Transform = {}
do
    Transform.new = hv_math.create_transform_object
end

return {
    Position2 = Position2,
    Velocity2 = Velocity2,
    Transform = Transform,
}