-- A simple port of https://simplegametutorials.github.io/love/asteroids/ for the purpose of testing
-- Heavy as a Love2D-like engine.

local gfx = hf.graphics

function hv.load()
    arenaWidth = 800
    arenaHeight = 680

    shipRadius = 30

    bulletTimerLimit = 0.5
    bulletRadius = 5

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
        shipX = arenaWidth / 2
        shipY = arenaHeight / 2
        shipAngle = 0
        shipSpeedX = 0
        shipSpeedY = 0

        bullets = {}
        bulletTimer = bulletTimerLimit

        asteroids = {
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

        for asteroidIndex, asteroid in ipairs(asteroids) do
            -- asteroid.angle = love.math.random() * (2 * math.pi)
            asteroid.angle = math.random() * (2 * math.pi)
            asteroid.stage = #asteroidStages
        end
    end

    reset()
end

function hv.update(dt)
    local turnSpeed = 10

    if hf.keyboard.is_down('right') then
         shipAngle = shipAngle - turnSpeed * dt
    end

    if hf.keyboard.is_down('left') then
        shipAngle = shipAngle + turnSpeed * dt
    end

    shipAngle = shipAngle % (2 * math.pi)

    if hf.keyboard.is_down('up') then
        local shipSpeed = 100
        shipSpeedX = shipSpeedX + math.cos(shipAngle) * shipSpeed * dt
        shipSpeedY = shipSpeedY + math.sin(shipAngle) * shipSpeed * dt
    end

    shipX = (shipX + shipSpeedX * dt) % arenaWidth
    shipY = (shipY + shipSpeedY * dt) % arenaHeight

    local function areCirclesIntersecting(aX, aY, aRadius, bX, bY, bRadius)
        return (aX - bX)^2 + (aY - bY)^2 <= (aRadius + bRadius)^2
    end

    for bulletIndex = #bullets, 1, -1 do
        local bullet = bullets[bulletIndex]

        bullet.timeLeft = bullet.timeLeft - dt
        if bullet.timeLeft <= 0 then
            table.remove(bullets, bulletIndex)
        else
            local bulletSpeed = 500
            bullet.x = (bullet.x + math.cos(bullet.angle) * bulletSpeed * dt)
                % arenaWidth
            bullet.y = (bullet.y + math.sin(bullet.angle) * bulletSpeed * dt)
                % arenaHeight

            for asteroidIndex = #asteroids, 1, -1 do
                local asteroid = asteroids[asteroidIndex]

                if areCirclesIntersecting(
                    bullet.x, bullet.y, bulletRadius,
                    asteroid.x, asteroid.y,
                    asteroidStages[asteroid.stage].radius
                ) then
                    table.remove(bullets, bulletIndex)

                    if asteroid.stage > 1 then
                        -- local angle1 = love.math.random() * (2 * math.pi)
                        local angle1 = math.random() * (2 * math.pi)
                        local angle2 = (angle1 - math.pi) % (2 * math.pi)

                        table.insert(asteroids, {
                            x = asteroid.x,
                            y = asteroid.y,
                            angle = angle1,
                            stage = asteroid.stage - 1,
                        })
                        table.insert(asteroids, {
                            x = asteroid.x,
                            y = asteroid.y,
                            angle = angle2,
                            stage = asteroid.stage - 1,
                        })
                    end

                    table.remove(asteroids, asteroidIndex)
                    break
                end
            end
        end
    end

    bulletTimer = bulletTimer + dt

    if hf.keyboard.is_down('s') then
        if bulletTimer >= bulletTimerLimit then
            bulletTimer = 0

            table.insert(bullets, {
                x = shipX + math.cos(shipAngle) * shipRadius,
                y = shipY + math.sin(shipAngle) * shipRadius,
                angle = shipAngle,
                timeLeft = 4,
            })
        end
    end

    for asteroidIndex, asteroid in ipairs(asteroids) do
        asteroid.x = (asteroid.x + math.cos(asteroid.angle)
            * asteroidStages[asteroid.stage].speed * dt) % arenaWidth
        asteroid.y = (asteroid.y + math.sin(asteroid.angle)
            * asteroidStages[asteroid.stage].speed * dt) % arenaHeight

        if areCirclesIntersecting(
            shipX, shipY, shipRadius,
            asteroid.x, asteroid.y, asteroidStages[asteroid.stage].radius
        ) then
            reset()
            break
        end
    end

    if #asteroids == 0 then
        reset()
    end
end

function hv.draw()
    gfx.origin()

    for y = -1, 1 do
        for x = -1, 1 do
            gfx.origin()
            gfx.translate(x * arenaWidth, y * arenaHeight)

            gfx.set_color(0, 0, 1)
            gfx.circle(gfx.DrawMode.Fill, shipX, shipY, shipRadius)

            local shipCircleDistance = 20
            gfx.set_color(0, 1, 1)
            gfx.circle(
                gfx.DrawMode.Fill,
                shipX + math.cos(shipAngle) * shipCircleDistance,
                shipY + math.sin(shipAngle) * shipCircleDistance,
                5
            )

            for bulletIndex, bullet in ipairs(bullets) do
                gfx.set_color(0, 1, 0)
                gfx.circle(gfx.DrawMode.Fill, bullet.x, bullet.y, bulletRadius)
            end

            for asteroidIndex, asteroid in ipairs(asteroids) do
                gfx.set_color(1, 1, 0)
                gfx.circle(gfx.DrawMode.Fill, asteroid.x, asteroid.y,
                    asteroidStages[asteroid.stage].radius)
            end
        end
    end
end
