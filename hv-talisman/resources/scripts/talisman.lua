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

    Class.get_class = talisman.components.class.get
    Class.set_class = talisman.components.class.set
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

    Name.get_name = talisman.components.name.get
    Name.set_name = talisman.components.name.set
end

talisman.get_editor = hv.plugins.talisman.editor.get_editor

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

talisman.editor = {}

local get_current_editor_level = hv.plugins.talisman.editor.get_current_level
function talisman.editor.get_current_level()
    return Level:new(get_current_editor_level())
end

function talisman.editor.get_current_space()
    return Space:new(talisman.editor.get_current_level()._space)
end

function talisman.editor.save_current_level_to_path(path)
    talisman.editor.get_current_level():save(path)
end

function talisman.editor.open_level_from_path(path)
    hv.plugins.talisman.editor.open_level(Level:load(path)._level)
end

return talisman