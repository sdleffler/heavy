local hf_collision = assert(hv.plugins.friends.collision)

local collision = {}
do
    collision.intersection_test = hf_collision.intersection_test

    local Collider = {}
    do
        collision.Collider = Collider

        Collider.ball = hf_collision.create_ball
        Collider.compound = hf_collision.create_compound
        Collider.cuboid = hf_collision.create_cuboid
        Collider.halfspace = hf_collision.create_halfspace
        Collider.convex_hull = hf_collision.create_convex_hull
        Collider.convex_polyline = hf_collision.create_convex_polyline
        Collider.polyline = hf_collision.create_polyline
        Collider.segment = hf_collision.create_segment
    end
end

return collision
