local hf = require("hf")
local danmaku = require("danmaku")
local LinearVelocity = danmaku.LinearVelocity
local PolarVelocity = danmaku.PolarVelocity
local StateMachine = danmaku.StateMachine

local danmaku_context = danmaku.Danmaku:new(space)

local barrage = danmaku.Barrage:new(danmaku_context)

local sprite_batch = danmaku.load_colorless_sprite("/sprites/bullet.png", "/sprites/bullet.json")
local sprites = {
    ["small"] = danmaku.ProjectileSprite(sprite_batch, "small"),
    ["small-spawn"] = danmaku.ProjectileSprite(sprite_batch, "small-spawn", false),
    nil
}

local arc_attack
do
    local decelerator = danmaku.sm.lerp_linear_speed(6., 0.8, 0.35)
    local shot = danmaku.shot_type.from_component_fn(function()
        return LinearVelocity, PolarVelocity, StateMachine(decelerator)
    end)

    local tx = hf.math.Isometry2.new(120, 120, math.pi)
    local polar_tx = hf.math.Isometry2.new(10, 0, 0)
    local linear_vel = hf.math.Velocity2.new(30, 0, 0)
    local count = 20

    function test_attack(barrage)
        barrage:set_shot_type(shot)
        barrage:append_origin(tx)
        barrage:append_polar_tx(polar_tx)
        barrage:add_linear_velocity(linear_vel)
        barrage:fire()
    end

    arc_attack = danmaku.Pattern:new(test_attack)
        :of(danmaku.Arc:new(math.pi / 3, 15, count))
        :indexed(function(indexer)
            return danmaku.Pattern:new(function(barrage)
                local polar_vel = hf.math.Velocity2.new(0.2, 0, 4 * math.pi * (indexer.index / count))
                local mod3, r, g, b = indexer.index % 3, 0.5, 0.5, 0.5
        
                if mod3 == 0 then
                    r = 1
                elseif mod3 == 1 then
                    g = 1
                elseif mod3 == 2 then
                    b = 1
                end        

                barrage:push()
                barrage:add_polar_velocity(polar_vel)
                barrage:set_color(r, g, b, 1)
                barrage:fire()
                barrage:pop()
            end)
        end)
end

local ring_attack
do
    local sprite_batch = danmaku.load_colorless_sprite("/sprites/bullet.png", "/sprites/bullet.json")
    local sprites = {
        ["small"] = danmaku.ProjectileSprite(sprite_batch, "small"),
        ["small-spawn"] = danmaku.ProjectileSprite(sprite_batch, "small-spawn", false),
        nil
    }

    local sm = StateMachine(danmaku.sm.parallel(
        danmaku.sm.lerp_linear_speed(4., 1.5, 0.5),
        danmaku.sm.lerp_polar_linear_speed(2., 0., 3.),
        danmaku.sm.lerp_polar_angular_speed(2., 1., 3.),
        danmaku.sm.sprite_sequence(
            sprites["small-spawn"],
            sprites["small"]
        )
    ))

    local shot = danmaku.shot_type.from_component_fn(function()
        return LinearVelocity, PolarVelocity, sm
    end)

    local tx = hf.math.Isometry2.new(120, 120, 0)
    local polar_vel = hf.math.Velocity2.new(16, 0, math.pi / 4)
    -- local polar_tx = hf.math.Isometry2.new(15, 0, 0)
    local linear_vel = hf.math.Velocity2.new(15, 0, math.pi * 1.5)
    local count = 45
    
    function test_attack(barrage)
        barrage:set_shot_type(shot)
        barrage:append_origin(tx)
        -- barrage:add_polar_tx(polar_tx)
        barrage:fire()
    end
    
    ring_attack = danmaku.Pattern:new(test_attack)
        :of(danmaku.Ring:new(15, count))
        :indexed(function(indexer)
            return danmaku.Pattern:new(function(barrage)
                local polar_offset = hf.math.Isometry2.new(15, 0, math.pi * (6 * indexer.index / count))
                local mod3, r, g, b = indexer.index % 3, 0.5, 0.5, 0.5
        
                if mod3 == 0 then
                    r = 1
                elseif mod3 == 1 then
                    r = 1
                    g = 1
                elseif mod3 == 2 then
                    b = 1
                end        
    
                barrage:push()
                barrage:add_linear_velocity(linear_vel)
                barrage:add_polar_velocity(polar_vel)
                barrage:add_polar_tx(polar_offset)
                barrage:set_color(r, g, b, 1)
                barrage:fire()
                barrage:pop()
            end)
        end)
end

-- arc_attack:build(barrage)
ring_attack:build(barrage)
barrage:flush()

function hv.update(dt)
    danmaku_context:update(dt)
end

function hv.draw()
    danmaku_context:draw()
end