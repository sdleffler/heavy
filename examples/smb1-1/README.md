# SMB1-1 example

A rough recreation of a classic level from a classic game.

## Notes

There are a number of things we can note about the implementation which might be relevant to someone
building a game using Heavy. Here's a short summary of the important things we came across and did
while building it.

### "Discrete" (non-continuous) collision detection/resolution

The collision handling in this demo is susceptible to a common/known bug called "tunneling".
Tunneling happens when you have a collision system that checks for collisions *after* moving an
object to its new position, rather than using a "continuous" detection method which robustly checks
all along the path the object will move on that given timestep. Continuous collision detection *can*
be quite complex, but is often not necessary - in order to "tunnel", an object has to be moving fast
enough at an object which is thin enough to completely skip over it in a single frame. In our demo,
the only tile which is thin enough to be susceptible to this is the flagpole, and it's impossible to
run at it *that* fast in the level as it's built. So here, it's ignored. If you want to implement
your own continuous collision detection system w/ AABBs though, you can use a "swept" AABB, which is
quite simple: you merge the current AABB of the object with the AABB of the object one timestep
forward (where it *wants* to be) and then check collisions with that. The enlarged bounding box
contains all possible intermediate positions of the object, so it won't skip any collisions... But
could cause false positives. It's a tricky subject, which is why we decided to go with the simple
solution that shouldn't be able to cause issues in the current context.

### Sprite padding

Sprites here are done using Aseprite and generating packed Aseprite JSON files. There are a few
gotchas; due to floating point error, it is possible for the "next pixel over" from one frame of a
sprite to be briefly visible. This is a pretty common issue w/ texture atlases known as "bleed". For
this reason, the spritesheets are generated with one pixel of padding. This causes Aseprite to pack
with at minimum one pixel of transparency between each frame of the sprite, and eliminates the pixel
bleed problem.