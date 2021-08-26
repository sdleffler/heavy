local hf_math = hv.plugins.friends.math

local Isometry2 = {}
do
    Isometry2.new = hf_math.create_isometry2_object
    Isometry2.identity = hf_math.create_isometry2_object_from_identity
    
    function Isometry2.from_translation(x, y)
        return Isometry2.new(x, y, 0)
    end

    function Isometry2.from_rotation(angle)
        return Isometry2.new(0, 0, angle)
    end

    setmetatable(Isometry2, {
        __call = function(_, x, y, angle)
            return Isometry2.new(x or 0, y or 0, angle or 0)
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

return {
    Isometry2 = Isometry2,
    Velocity2 = Velocity2,
}