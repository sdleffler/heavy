local binser = require("std.binser")

local talisman = {}

local Class = {}
do
    talisman.Class = Class

    local talisman = hv.plugins.talisman

    local create_constructor = talisman.components.class.create_constructor
    setmetatable(Class, { __call = function(_, class) return create_constructor(class) end })

    Class.class_get = talisman.components.class.get
    Class.class_set = talisman.components.class.set
end

local Name = {}
do
    talisman.Name = Name

    local talisman = hv.plugins.talisman

    local create_constructor = talisman.components.name.create_constructor
    setmetatable(Name, { __call = function(_, name) return create_constructor(name) end })

    Name.name_get = talisman.components.name.get
    Name.name_set = talisman.components.name.set
end

return talisman
