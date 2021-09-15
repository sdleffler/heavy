local Goomba = require("smb1_1.level_encoding").Goomba
local Koopa = require("smb1_1.level_encoding").Koopa
local Player = require("smb1_1.level_encoding").Player

local level = hv.spaces.create_space()

-- Width of a single tile in pixels.
local tile_px = 16

-- Load map?

function goomba(x, y)
    return Goomba:new(level, (x + 0.5) * tile_px, (y + 0.5) * tile_px)
end

function koopa(x, y)
    return Koopa:new(level, (x + 0.5) * tile_px, (y + 0.5) * tile_px)
end

Player:new(level, 2.5 * tile_px, 2.5 * tile_px)

-- First goomba, in between the two ? blocks near the start.
goomba(22, 2)

-- Second goomba, in between the second and third pipes.
goomba(41, 2)

-- Third+fourth goombi, in between the third and fourth pipes.
goomba(53, 2)
goomba(54, 2)

-- Fifth+sixth goombi, on the high platform.
goomba(83, 10)
goomba(85, 10)

-- Seventh+eighth goombi, in between the coin block and star block and just before the first koopa.
goomba(102, 2)
goomba(103.5, 2)

-- First koopa, one tile to the right of the first ? block in the triangle.
koopa(113, 2)

-- Ninth+tenth goombi, first of three pairs before the staircases
goomba(120, 2)
goomba(121.5, 2)

-- Eleventh+twelfth
goomba(131, 2)
goomba(132.5, 2)

-- Thirteenth+fourteenth
goomba(135, 2)
goomba(136.5, 2)

-- Fifteenth+sixteenth and final.
goomba(179, 2)
goomba(180.5, 2)

return level