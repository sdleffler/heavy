local std_agent = require("std.agent")
local Agent, State = std_agent.Agent, std_agent.State

local revival_length         = 10.0
local almost_reviving_length = 8.0 

local revive_timer = 0.0
-- Need to set these once they actually move
local walk_speed = 0
local shell_speed = 0
local dt = 1.0/60.0

local tag_walk     = assert(rust.sprite_sheets.koopa:get_tag("walk"))
local tag_in_shell = assert(rust.sprite_sheets.koopa:get_tag("shell_spin"))
local tag_reviving = assert(rust.sprite_sheets.koopa:get_tag("reviving"))

local Walking = State:extend("smb1_1.koop.walking", { name = "walk" })
do
    function Walking:update(agent, koopa)
        koopa:sprite_animation_update(1.0/60.0)
        -- TODO: if stomp, then move to shell stop
    end
end

local ShellStop = State:extend("smb1_1.koopa.ShellStop", { name = "shell_stop" })
    function ShellStop:update(agent, koopa)
        revive_timer = revive_timer + 1

	-- if the koopa is about to revive, swap the reviving animation with the regular one
	-- every other frame
	if (revive_timer / 60) >= almost_reviving_length then
            if revive_timer % 2 then
		koopa.tag = tag_reviving
	    else
		koopa.tag = tag_in_shell
	    end
        end

        if (revive_timer / 60) >= revive_speed then
            agent:push("walk")
	    koopa.tag = tag_walk
	    revive_timer = 0.0
        end

	-- TODO: need to check for collision and enter shell drift state
    end
do
    
end

local ShellDrift = State:extend("smb1_1.koopa.ShellDrift", { name = "shell_drift" })
    function ShellDrift:update(agent, koopa)
        -- Implement the shell sliding
	-- Implement going back to shell stop after being bounced on
    end
do
    
end

local KoopaController = Agent:extend("KoopaController")
do
    KoopaController:add_states {
        Walking,
        ShellStop,
	ShellDrift,
    }

    KoopaController:bind {
        "update",
    }
end

return {
    KoopaController = KoopaController,
}