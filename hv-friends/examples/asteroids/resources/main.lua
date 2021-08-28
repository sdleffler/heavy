local hf = require("hf")
local gfx = hf.graphics

function hv.update(dt)
    -- print(dt)
end

function hv.draw()
    gfx.clear()
    
    gfx.print("Hello world!", 100, 100, 0, 4)

    gfx.present()
end
