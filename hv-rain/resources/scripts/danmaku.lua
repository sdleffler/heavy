local class = require("std.class")
local hf = require("hf")
local Isometry2 = hf.math.Isometry2
local Velocity2 = hf.math.Velocity2
local hv_danmaku = hv.plugins.danmaku

local shot_type = {
    from_component_fn = hv_danmaku.create_shot_type_from_component_fn
}

local function load_sprite(img_path, sheet_path, pipeline)
    local texture = hf.graphics.load_texture_from_filesystem(img_path)
    local sheet = hf.graphics.load_sprite_sheet_from_filesystem(sheet_path)
    return hv_danmaku.create_projectile_sprite_batch(texture, sheet, pipeline)
end

local Danmaku = class("Danmaku")

function Danmaku:init(space)
    self._danmaku = hv_danmaku.create_danmaku_object(space)
end

function Danmaku:update(dt)
    self._danmaku:update(dt)
end

function Danmaku:draw()
    self._danmaku:draw()
end

local Barrage = class("Barrage")

function Barrage:new(danmaku)
    assert(class.isInstance(danmaku) and danmaku:instanceOf(Danmaku),
        "Barrage must be instantiated with respect to a Danmaku object")
    local raw_danmaku_object = danmaku._danmaku:create_barrage_object()
    local self = Barrage:create()
    self:init(raw_danmaku_object)
    return self
end

function Barrage:init(inner)
    assert(inner, "barrage must be instantiated with non-nil inner object")
    self._inner = inner
end

local barrage_0arg_keys = {
    "push",
    "pop",
    "fire",
    "flush"
}

local barrage_1arg_keys = {
    "set_lua_value",
    "prepend_origin",
    "append_origin",
    "prepend_linear_tx",
    "append_linear_tx",
    "set_linear_tx",
    "add_polar_tx",
    "set_polar_tx",
    "add_linear_velocity",
    "set_linear_velocity",
    "add_linear_velocity_wrt_world",
    "set_linear_velocity_wrt_world",
    "add_polar_velocity",
    "set_polar_velocity",
    "add_linear_acceleration",
    "set_linear_acceleration",
    "add_linear_acceleration_wrt_world",
    "set_linear_acceleration_wrt_world",
    "add_polar_acceleration",
    "set_polar_acceleration",
    "set_shot_type",
    "set_sprite",
    nil
}

local barrage_4arg_keys = {
    "set_color"
}

for _,key in ipairs(barrage_0arg_keys) do
    Barrage[key] = function(self)
        local _inner = self._inner
        _inner[key](_inner)
    end
end

for _,key in ipairs(barrage_1arg_keys) do
    Barrage[key] = function(self, a)
        local _inner = self._inner
        _inner[key](_inner, a)
    end
end

for _,key in ipairs(barrage_4arg_keys) do
    Barrage[key] = function(self, a, b, c, d)
        local _inner = self._inner
        _inner[key](_inner, a, b, c, d)
    end
end

local Compose
do
    Compose = Barrage:extend("Compose")

    function Compose:init(pattern, barrage)
        Compose.super.init(self, barrage)
        self._pattern = pattern
    end

    function Compose:fire()
        self._pattern:build(self._inner)
    end
end

local Pattern, Of
local Indexed, Indexer
do
    Pattern = class("Pattern")

    function Pattern:init(closure)
        if closure then
            self.build = function(_, barrage)
                closure(barrage)
            end
        end
    end

    function Pattern:build(barrage) end

    function Pattern:of(subpattern)
        return Of:new(self, subpattern)
    end

    function Pattern:indexed(f)
        return Indexed:new(self, f)
    end

    Of = Pattern:extend("Of")

    function Of:init(a, b)
        self._a = a
        self._b = b
        self._compose = Compose:create()
    end

    function Of:build(barrage)
        local compose = self._compose
        compose:init(self._b, barrage)
        self._a:build(compose)
    end

    Indexer = Compose:extend("Indexer", { index = 0 })

    function Indexer:init(pattern, barrage)
        Indexer.super.init(self, pattern, barrage)
        self.index = 0
    end

    function Indexer:fire()
        self._pattern:build(self._inner)
        self.index = self.index + 1
    end

    Indexed = Of:extend("Indexed")

    function Indexed:init(a, f)
        self._a = a

        local indexer = Indexer:create()
        self._b = f(indexer)
        self._compose = indexer
    end
end

local Arc
do
    Arc = Pattern:extend("Arc")

    function Arc:init(angle, radius, count)
        assert(count > 0, "arc must have a nonzero number of shots")

        self._angle = angle

        if count > 1 then
            self._initial = Isometry2.new(0, radius, -angle / 2.)
        else
            self._initial = Isometry2.from_translation(0, radius)
        end
        
        self._count = count
        self._iso = Isometry2.from_rotation(0)
    end

    function Arc:build(barrage)
        barrage:push()
        barrage:append_origin(self._initial)
        local iso, angle, count = self._iso, self._angle, self._count
        for i=1,self._count do
            iso.angle = ((i - 1) / count) * angle
            barrage:push()
            barrage:append_origin(iso)
            barrage:fire()
            barrage:pop()
        end
        barrage:pop()
    end
end

local Ring
do
    Ring = Pattern:extend("Ring")

    function Ring:init(radius, count)
        assert(count > 0, "arc must have a nonzero number of shots")

        self._iso = Isometry2.from_rotation(0)
        self._initial = Isometry2.from_translation(0, radius)
        self._count = count
    end

    function Ring:build(barrage)
        barrage:push()
        barrage:append_origin(self._initial)
        local iso, count = self._iso, self._count
        for i=1,self._count do
            iso.angle = ((i - 1) / count) * 2 * math.pi
            barrage:push()
            barrage:append_origin(iso)
            barrage:fire()
            barrage:pop()
        end
        barrage:pop()
    end
end

return {
    shot_type = shot_type,
    sm = require("danmaku.sm"),

    Danmaku = Danmaku,

    Barrage = Barrage,
    Pattern = Pattern,

    Arc = Arc,
    Ring = Ring,

    LinearVelocity = hv_danmaku.linear_velocity_component_constructor,
    PolarVelocity = hv_danmaku.polar_velocity_component_constructor,
    StateMachine = hv_danmaku.state_machine_component_constructor,
    ProjectileSprite = hv_danmaku.projectile_sprite_component_constructor,

    load_sprite = load_sprite,
    load_colorless_sprite = function(img_path, sheet_path)
        return load_sprite(img_path, sheet_path, hv_danmaku.get_color_bullet_pipeline())
    end,

    nil
}