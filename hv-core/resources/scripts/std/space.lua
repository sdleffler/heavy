local class = require("std.class")
local binser = require("std.binser")
local ObjectTable = hv.components.ObjectTable

local space = {}
do
    local Space = class("Space")
    do
        space.Space = Space

        local all_spaces = setmetatable({}, { __mode = "kv" })

        function Space:new(space)
            return space and all_spaces[space:id()] or
                Space:create():init(space)
        end

        function Space:init(space)
            self._space = space or hv.spaces.create_space()
            all_spaces[self._space:id()] = self
            return self
        end

        function Space:objects()
            return self._space:objects()
        end
    end

    local Object = class("Object")
    do
        space.Object = Object
        binser.registerClass(Object)

        function Object:init(space, ...)
            space._space:spawn(ObjectTable(self), ...)
        end
    end
end

return space