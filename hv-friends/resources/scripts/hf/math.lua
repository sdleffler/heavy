local hf_math = assert(hv.plugins.friends.math)

local Position2 = {}
do
    Position2.new = assert(hf_math.create_position2_object)
    Position2.identity = assert(hf_math.create_position2_object_from_identity)

    function Position2.from_translation(x, y) return Position2.new(x, y, 0) end
    function Position2.from_rotation(angle) return Position2.new(0, 0, angle) end

    setmetatable(
        Position2,
        { __call = function(_, x, y, angle) return Position2.new(x or 0, y or 0, angle or 0) end }
    )
end

local Velocity2 = {}
do
    Velocity2.new = assert(hf_math.create_velocity2_object)
    Velocity2.zero = assert(hf_math.create_velocity2_object_from_zero)

    function Velocity2.from_linear(x, y) return Velocity2.new(x, y, 0) end
    function Velocity2.from_angular(angular) return Velocity2.new(0, 0, angular) end

    setmetatable(
        Velocity2, {
            __call = function(_, x, y, angular)
                return Velocity2.new(x or 0, y or 0, angular or 0)
            end,
        }
    )
end

local Box2 = {}
do
    Box2.new = assert(hf_math.create_box2_from_extents)
    Box2.invalid = assert(hf_math.create_box2_invalid)
    Box2.huge = assert(hf_math.create_box2_huge)
    Box2.from_corners = assert(hf_math.create_box2_from_corners)
    Box2.from_extents = assert(hf_math.create_box2_from_extents)
    Box2.from_half_extents = assert(hf_math.create_box2_from_half_extents)

    setmetatable(
        Box2,
        { __call = function(_, x, y, w, h) return Box2.new(x or 0, y or 0, w or 0, h or 0) end }
    )
end

local Transform = {}
do
    Transform.new = assert(hf_math.create_transform_identity)
    Transform.identity = assert(Transform.new)
    Transform.isometry2 = assert(hf_math.create_transform_isometry2)
    Transform.rotation2 = assert(hf_math.create_transform_rotation2)
    Transform.translation2 = assert(hf_math.create_transform_translation2)
end

return { Box2 = Box2, Position2 = Position2, Velocity2 = Velocity2, Transform = Transform }
