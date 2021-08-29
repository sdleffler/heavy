-- A simple port of https://simplegametutorials.github.io/love/asteroids/ for the purpose of testing
-- Heavy as a Love2D-like engine.

local class = require("std.class")
local gfx = hf.graphics
local ObjectTable = hv.components.ObjectTable
local Position = hf.components.Position
local Velocity = hf.components.Velocity

-- From Rust, in our game init code.
local Circle = asteroids_rust.make_circle
local AsteroidMarker = asteroids_rust.make_asteroid
local PlayerMarker = asteroids_rust.make_player
local BulletMarker = asteroids_rust.make_bullet
local space = asteroids_rust.space
local arenaWidth, arenaHeight = asteroids_rust.arenaWidth, asteroids_rust.arenaHeight

local Asteroid = class("Asteroid")
    :with(Position)

function Asteroid:init(x, y, angle, stage)
    self.stage = stage
    local stageTable = asteroidStages[stage]
    local speed, radius = stageTable.speed, stageTable.radius
    local vx, vy = math.cos(angle) * speed, math.sin(angle) * speed

    space:spawn(
        ObjectTable(self),
        Position(x, y, angle),
        Velocity(vx, vy),
        Circle(asteroidStages[stage].radius, 1, 1, 0),
        AsteroidMarker
    )
end

function Asteroid:destroy()
    local stage = self.stage

    if stage > 1 then
        -- From `Position` mixin.
        local x, y = self:position_get_coords()

        local angle1 = math.random() * (2 * math.pi)
        local angle2 = (angle1 - math.pi) % (2 * math.pi)

        Asteroid:new(x, y, angle1, stage - 1)
        Asteroid:new(x, y, angle2, stage - 1)
    end

    space:despawn(self)
end

local Ship = class("Ship")
    :with(Position)
    :with(Velocity)

function Ship:init()
    space:spawn(
        ObjectTable(self),
        Position(arenaWidth / 2, arenaHeight / 2, 0),
        Velocity(0, 0, 0),
        Circle(30, 0, 0, 1),
        PlayerMarker
    )
end

function Ship:fire()
    -- From `Position` mixin.
    local tx = self:position_get():to_transform()
    local bullet_speed = 500

    -- Form the bullet using position and velocities relative to the ship.
    space:spawn(
        Position(tx:transform_point2(20, 0)),
        Velocity(tx:transform_vector2(bullet_speed, 0)),
        Circle(5, 0, 1, 0),
        BulletMarker(4)
    )
end

function Ship:destroy()
    reset()
end

function hv.load()
    bulletTimerLimit = 0.5

    asteroidStages = {
        {
            speed = 120,
            radius = 15,
        },
        {
            speed = 70,
            radius = 30,
        },
        {
            speed = 50,
            radius = 50,
        },
        {
            speed = 20,
            radius = 80,
        },
    }

    function reset()
        space:clear()

        ship = Ship:new()

        bulletTimer = bulletTimerLimit

        local asteroids = {
            {
                x = 100,
                y = 100,
            },
            {
                x = arenaWidth - 100,
                y = 100,
            },
            {
                x = arenaWidth / 2,
                y = arenaHeight - 100,
            }
        }

        for _, asteroid in ipairs(asteroids) do
            Asteroid:new(asteroid.x, asteroid.y, math.random() * (2 * math.pi), #asteroidStages)
        end
    end

    reset()
end

function hv.update(dt)
    local turnSpeed = 10
    
    if hf.keyboard.is_down('right') then
        -- From `Position` mixin.
        ship:position_add_angle(-turnSpeed * dt)
    end

    if hf.keyboard.is_down('left') then
        -- From `Position` mixin.
        ship:position_add_angle(turnSpeed * dt)
    end

    -- From `Position` mixin.
    shipAngle = ship:position_get_angle()

    if hf.keyboard.is_down('up') then
        local shipSpeed = 100
        -- From `Velocity` mixin.
        ship:velocity_add_linear(
            math.cos(shipAngle) * shipSpeed * dt,
            math.sin(shipAngle) * shipSpeed * dt
        )
    end

    bulletTimer = bulletTimer + dt

    if hf.keyboard.is_down('s') then
        if bulletTimer >= bulletTimerLimit then
            bulletTimer = 0
            ship:fire()
        end
    end
end

function hv.draw()
    -- All rendering is done speedily in Rust.
end
