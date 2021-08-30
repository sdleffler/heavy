local space = require("std.space")
local binser = require("std.binser")
local Space = space.Space

local talisman = {}

local Class = {}
do
    talisman.Class = Class

    local talisman = hv.plugins.talisman

    local create_constructor = talisman.components.class.create_constructor
    setmetatable(Class, {
        __call = function(_, class)
            return create_constructor(class)
        end
    })

    Class.class_get = talisman.components.class.get
    Class.class_set = talisman.components.class.set
end

local Name = {}
do
    talisman.Name = Name

    local talisman = hv.plugins.talisman

    local create_constructor = talisman.components.name.create_constructor
    setmetatable(Name, {
        __call = function(_, name)
            return create_constructor(name)
        end
    })

    Name.name_get = talisman.components.name.get
    Name.name_set = talisman.components.name.set
end

local Level = space.Space:extend("Level")
do
    talisman.Level = Level

    function Level:new(level)
        local this = Level:create()
        this:init(level)
        return this
    end

    function Level:init(level)
        Level.super.init(self, level.space)
        self._level = level
    end

    function Level:save(path)
        self._level:save(path)
    end

    function Level:load(path)
        return Level:new(hv.plugins.talisman.level.load_level_from_path(path))
    end
end

return talisman